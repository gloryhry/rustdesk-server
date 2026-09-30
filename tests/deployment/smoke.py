"""Run in an isolated helper sharing the API network namespace; no production use."""
import argparse
import base64
import hashlib
import http.server
import json
import os
import socket
import threading
import time
import urllib.error
import urllib.parse
import urllib.request
import uuid


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *args):
        return None


def varint(value):
    result = bytearray()
    while value > 127:
        result.append((value & 127) | 128)
        value >>= 7
    return bytes(result + bytes([value]))


def field(number, payload):
    return varint(number * 8 + 2) + varint(len(payload)) + payload


def decode_fields(data):
    offset = 0
    def read_varint():
        nonlocal offset
        result = 0
        for shift in range(0, 70, 7):
            byte = data[offset]; offset += 1; result |= (byte & 127) << shift
            if byte < 128:
                return result
        raise AssertionError('invalid protobuf response')
    result = {}
    while offset < len(data):
        tag = read_varint()
        if tag & 7 == 0:
            result[tag >> 3] = read_varint()
        elif tag & 7 == 2:
            length = read_varint(); result[tag >> 3] = data[offset:offset + length]; offset += length
        else:
            raise AssertionError('unexpected protobuf wire type')
    return result


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--url', default='http://127.0.0.1:21114')
    parser.add_argument('--hbbs', default='127.0.0.1:21116')
    parser.add_argument('--fixtures', default='/smoke/requests.json')
    parser.add_argument('--expected-guid')
    args = parser.parse_args()
    opener = urllib.request.build_opener(NoRedirect)
    token = None

    def request(method, path, body=None, auth=True, cookie=None):
        headers = {'Content-Type': 'application/json'}
        if auth and token:
            headers['Authorization'] = 'Bearer ' + token
        if cookie:
            headers['Cookie'] = cookie
        payload = None if body is None else json.dumps(body).encode()
        if method in ('POST', 'PUT', 'DELETE') and payload is None:
            payload = b''
        try:
            response = opener.open(urllib.request.Request(args.url + path, data=payload, method=method, headers=headers), timeout=10)
        except urllib.error.HTTPError as response_error:
            response = response_error
        raw = response.read()
        try:
            value = json.loads(raw)
        except ValueError:
            value = raw.decode()
        return response.status, value, response.headers

    assert request('GET', '/health/ready')[0] == 200
    status, html, _ = request('GET', '/')
    assert status == 200 and '<script' in html and '/assets/' in html
    status, login, headers = request('POST', '/api/login', {'username': os.environ['API_BOOTSTRAP_ADMIN_USERNAME'], 'password': os.environ['API_BOOTSTRAP_ADMIN_PASSWORD']}, False)
    assert status == 200
    for attribute in ('Secure', 'HttpOnly', 'SameSite=Lax', 'Path=/'):
        assert attribute in headers['Set-Cookie']
    token = login['access_token']
    status, users, _ = request('GET', '/api/users?name=' + urllib.parse.quote(os.environ['API_BOOTSTRAP_ADMIN_USERNAME']))
    assert status == 200
    owner = next(user['id'] for user in users['data'] if user['name'] == os.environ['API_BOOTSTRAP_ADMIN_USERNAME'])
    status, personal, _ = request('POST', '/api/ab/personal')
    assert status == 200 and personal['guid']
    guid = personal['guid']
    if args.expected_guid:
        assert guid == args.expected_guid
        status, saved, _ = request('POST', '/api/ab/peers?ab=' + guid)
        assert status == 200 and any(peer['id'] == '777777' and peer['extension'] == {'persist': True} for peer in saved['data'])
        print(json.dumps({'persistent_guid': guid, 'result': 'passed'}))
        return

    with open(args.fixtures) as source:
        samples = json.load(source)
    for sample in samples['sequence']:
        status, result, _ = request(sample['method'], sample['path'].replace('$guid', guid), sample['body'])
        assert status == 200, (sample['path'], status)
        kind = sample['response']
        if kind == 'empty': assert result == ''
        elif kind == 'guid': assert result['guid'] == guid
        elif kind == 'settings': assert result['max_peer_one_ab'] == 0
        elif kind == 'empty_page': assert result == {'total': 0, 'data': []}
        elif kind == 'empty_tags': assert result == []
        elif kind == 'peer_page': assert result['total'] == 1 and result['data'][0]['alias'] == 'Renamed' and result['data'][0]['hash'] == 'test-only-hash'
        elif kind == 'tag_array': assert result == [{'name': 'work', 'color': 0x12345678}]

    # Use the real hbbs UDP registration protocol and then the unsigned official report.
    pk = bytes([3] * 32); peer_uuid = b'deployment-device'
    message = field(15, field(1, b'123456') + field(2, peer_uuid) + field(3, pk))
    host, port = args.hbbs.rsplit(':', 1)
    with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as udp:
        udp.settimeout(5); udp.sendto(message, (host, int(port)))
        response = decode_fields(udp.recv(65536)); assert 16 in response
        assert decode_fields(response[16]).get(1, 0) == 0
    status, result, _ = request('POST', '/api/sysinfo', {'id': '123456', 'uuid': base64.b64encode(peer_uuid).decode(), 'username': 'smoke', 'hostname': 'Isolated', 'os': 'Linux', 'version': '1.4.9'}, False)
    assert status == 200 and result == 'SYSINFO_UPDATED'
    status, _, _ = request('POST', '/api/admin/device/bind', {'peer_id': '123456', 'user_id': owner, 'pk_fingerprint': 'sha256:' + hashlib.sha256(pk).hexdigest()})
    assert status == 200
    status, peers, _ = request('GET', '/api/peers')
    assert status == 200 and any(peer['id'] == '123456' and isinstance(peer['info'], dict) for peer in peers['data'])
    status, registry, _ = request('GET', '/api/admin/device/registry?peer_id=123456')
    assert status == 200 and registry['data'][0]['untrusted_sysinfo']['hostname'] == 'Isolated'
    for _ in range(50):
        _, registry, _ = request('GET', '/api/admin/device/registry?peer_id=123456')
        if registry['data'][0]['registered_at_ms']:
            break
        time.sleep(.1)
    assert registry['data'][0]['registered_at_ms'] and registry['data'][0]['online']

    # Outbound token/userinfo calls are real loopback requests in the deployed API.
    class Mock(http.server.BaseHTTPRequestHandler):
        def log_message(self, *args):
            pass
        def do_POST(self):
            self.rfile.read(int(self.headers.get('Content-Length', 0)))
            self.send_response(200); self.send_header('Content-Type', 'application/json'); self.end_headers()
            self.wfile.write(b'{"access_token":"test-only-provider-token"}')
        def do_GET(self):
            self.send_response(200); self.send_header('Content-Type', 'application/json'); self.end_headers()
            self.wfile.write(b'{"id":"test-only-subject","login":"deployment-oauth"}')
    mock = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Mock)
    threading.Thread(target=mock.serve_forever, daemon=True).start()
    provider_name = 'deploy-' + uuid.uuid4().hex[:12]
    endpoint = 'http://127.0.0.1:' + str(mock.server_port)
    try:
        status, provider, _ = request('POST', '/api/admin/oauth/providers', {'name': provider_name, 'kind': 'oauth2', 'client_id': 'test-client', 'client_secret': 'test-only-secret', 'authorization_url': endpoint + '/authorize', 'token_url': endpoint + '/token', 'userinfo_url': endpoint + '/userinfo', 'scopes': 'read:user', 'enabled': True})
        assert status == 201 and 'client_secret' not in provider
        device = {'op': provider_name, 'id': '123456', 'uuid': 'deployment-native', 'deviceInfo': {'name': 'Isolated', 'os': 'Linux'}}
        status, launch, _ = request('POST', '/api/oidc/auth', device, False)
        assert status == 200 and set(launch) == {'code', 'url'}
        path = urllib.parse.urlsplit(launch['url']); path = path.path + '?' + path.query
        status, _, headers = request('GET', path, auth=False)
        assert status == 307
        binding = headers['Set-Cookie'].split(';', 1)[0]
        state = urllib.parse.parse_qs(urllib.parse.urlsplit(headers['Location']).query)['state'][0]
        status, _, headers = request('GET', '/api/oidc/callback?' + urllib.parse.urlencode({'state': state, 'code': 'test-only-code'}), auth=False, cookie=binding)
        assert status == 200 and 'rustdesk_api_token=' not in headers.get('Set-Cookie', '')
        status, claimed, _ = request('GET', '/api/oidc/auth-query?' + urllib.parse.urlencode({'code': launch['code'], 'id': device['id'], 'uuid': device['uuid']}), auth=False)
        assert status == 200 and claimed['access_token'] and isinstance(claimed['user']['info'], dict)
        assert request('POST', '/api/admin/oauth/providers/delete', {'id': provider['id']})[0] == 200
    finally:
        mock.shutdown(); mock.server_close()
    assert request('POST', '/api/ab/peer/add/' + guid, {'id': '777777', 'alias': 'Persistent', 'extension': {'persist': True}})[0] == 200
    print(json.dumps({'guid': guid, 'checks': ['readiness', 'static_web', 'secure_login', 'official_address_book', 'hbbs_registration', 'device_report', 'binding', 'online_observation', 'oauth_mock'], 'result': 'passed'}))


if __name__ == '__main__':
    main()
