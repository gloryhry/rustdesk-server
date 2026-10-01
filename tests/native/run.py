"""Provision disposable original Linux clients, HTTPS API and real Keycloak.

start leaves the desktop containers running for actual UI acceptance. collect
records evidence; cleanup removes only resources in this run's manifest.
"""
import argparse
import base64
import hashlib
import json
import os
from pathlib import Path
import re
import secrets
import socket
import subprocess
import time
import urllib.error
import urllib.request


def run(*args, input=None, check=True, include_stderr=False):
    result = subprocess.run(args, input=input, text=True, check=check,
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    return (result.stdout + (result.stderr if include_stderr else '')).strip()


def write(path, value, mode=0o600):
    path.write_text(value)
    path.chmod(mode)


def port():
    with socket.socket() as listener:
        listener.bind(('127.0.0.1', 0))
        return listener.getsockname()[1]


def wait(check, timeout=180):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        try:
            value = check()
            if value:
                return value
        except (OSError, ValueError, subprocess.CalledProcessError):
            pass
        time.sleep(1)
    raise RuntimeError('isolated service readiness timed out')


def api(manifest, path, method='GET', body=None, token=None):
    headers = {'Content-Type': 'application/json'}
    if token:
        headers['Authorization'] = 'Bearer ' + token
    request = urllib.request.Request(manifest['api_control_url'] + path,
                                    None if body is None else json.dumps(body).encode(), headers, method=method)
    with urllib.request.urlopen(request, timeout=10) as response:
        text = response.read().decode()
        try:
            return json.loads(text)
        except ValueError:
            return text


def start(args, root):
    if (root / 'manifest.json').exists():
        raise RuntimeError('directory already contains a run; use collect or cleanup')
    prefix = 'rustdesk-goal-' + secrets.token_hex(6)
    manifest = {'prefix': prefix, 'network': prefix, 'containers': [], 'volumes': [],
                'server_image': args.image, 'client_image': args.client_image,
                'api_control_url': f'http://127.0.0.1:{port()}', 'clients': {}, 'checks': {}}
    def save():
        write(root / 'manifest.json', json.dumps(manifest, indent=2))
    save()
    private = {'admin_password': secrets.token_urlsafe(24), 'account_password': secrets.token_urlsafe(24),
               'oidc_password': secrets.token_urlsafe(24), 'oidc_secret': secrets.token_urlsafe(32),
               'keycloak_password': secrets.token_urlsafe(24), 'remote_password': secrets.token_urlsafe(24)}
    write(root / 'credentials.json', json.dumps(private, indent=2))
    env = {'API_ENABLED': '1', 'API_BIND': '0.0.0.0', 'API_PUBLIC_URL': 'https://api.goal.test',
           'API_OAUTH_REDIRECT_URL': 'https://api.goal.test/api/oidc/callback',
           'API_JWT_SECRET': secrets.token_urlsafe(48),
           'API_OAUTH_CONFIG_KEY': base64.b64encode(secrets.token_bytes(32)).decode(),
           'API_BOOTSTRAP_ADMIN_USERNAME': 'goal-admin', 'API_BOOTSTRAP_ADMIN_PASSWORD': private['admin_password'],
           'DB_URL': '/data/db_v2.sqlite3', 'RUSTDESK_KEY_FILE': '/data/id_ed25519.pub',
           'API_WEB_ROOT': '/usr/share/rustdesk-api-web', 'SSL_CERT_FILE': '/goal/ca.crt',
           'RUSTDESK_ID_SERVER': 'hbbs:21116', 'RUSTDESK_RELAY_SERVER': 'hbbr:21117'}
    write(root / 'server.env', ''.join(k + '=' + v + '\n' for k,v in env.items()))
    run('openssl', 'req', '-x509', '-newkey', 'rsa:2048', '-nodes', '-days', '2',
        '-subj', '/CN=RustDesk Goal Test CA', '-keyout', str(root / 'ca.key'), '-out', str(root / 'ca.crt'))
    run('openssl', 'req', '-newkey', 'rsa:2048', '-nodes', '-subj', '/CN=api.goal.test',
        '-keyout', str(root / 'tls.key'), '-out', str(root / 'tls.csr'))
    write(root / 'tls.ext', 'subjectAltName=DNS:api.goal.test,DNS:idp.goal.test\nextendedKeyUsage=serverAuth\n')
    run('openssl', 'x509', '-req', '-in', str(root / 'tls.csr'), '-CA', str(root / 'ca.crt'),
        '-CAkey', str(root / 'ca.key'), '-CAcreateserial', '-days', '2', '-extfile', str(root / 'tls.ext'),
        '-out', str(root / 'tls.crt'))
    (root / 'ca.crt').chmod(0o644)
    for name in ['ca.key', 'tls.key']:
        (root / name).chmod(0o600)
    realm = {'realm': 'goal', 'enabled': True, 'sslRequired': 'external',
             'clients': [{'clientId': 'goal-client', 'enabled': True, 'protocol': 'openid-connect',
                          'publicClient': False, 'secret': private['oidc_secret'],
                          'redirectUris': ['https://api.goal.test/api/oidc/callback'],
                          'attributes': {'pkce.code.challenge.method': 'S256'}}],
             'users': [{'username': 'goal-oidc', 'enabled': True, 'emailVerified': True,
                        'email': 'oidc@goal.test', 'firstName': 'Goal', 'lastName': 'OIDC',
                        'credentials': [{'type': 'password', 'value': private['oidc_password'], 'temporary': False}]}]}
    write(root / 'realm.json', json.dumps(realm))
    write(root / 'keycloak.env', f'KC_BOOTSTRAP_ADMIN_USERNAME=goal-keycloak\nKC_BOOTSTRAP_ADMIN_PASSWORD={private["keycloak_password"]}\n')
    # Network and every resource name are recorded before creation for recovery.
    # A dedicated bridge permits loopback-published noVNC/control ports. Docker
    # internal networks suppress those port bindings on recent Engine versions.
    run('docker', 'network', 'create', prefix)
    data = prefix + '-data'
    manifest['volumes'].append(data)
    save()
    run('docker', 'volume', 'create', data)
    def container(suffix, image, options, command=(), detached=True):
        name = prefix + '-' + suffix
        manifest['containers'].append(name)
        save()
        run('docker', 'run', '-d' if detached else '--rm', '--name', name, '--network', prefix,
            *options, image, *command)
        return name
    server_options = ['--env-file', str(root / 'server.env'), '-v', data + ':/data',
                      '-v', str(root / 'ca.crt') + ':/goal/ca.crt:ro']
    container('init', args.image, server_options, ['init'], False)
    hbbr = container('hbbr', args.image, ['--network-alias', 'hbbr', *server_options], ['hbbr'])
    hbbs = container('hbbs', args.image, ['--network-alias', 'hbbs', *server_options],
                     ['hbbs', '--relay-servers', 'hbbr:21117'])
    service = container('api', args.image, ['--network-alias', 'api', *server_options], ['rustdesk-api'])
    manifest['services'] = {'hbbs': hbbs, 'hbbr': hbbr, 'api': service}
    keycloak = container('keycloak', 'quay.io/keycloak/keycloak:26.4.0',
        ['--network-alias', 'keycloak', '--env-file', str(root / 'keycloak.env'),
         '-v', str(root / 'realm.json') + ':/opt/keycloak/data/import/goal-realm.json:ro'],
        ['start-dev', '--import-realm', '--hostname=https://idp.goal.test', '--http-enabled=true', '--proxy-headers=xforwarded'])
    manifest['services']['keycloak'] = keycloak
    write(root / 'nginx.conf', '''events {}\nhttp {
      server { listen 80; location / { proxy_pass http://api:21114; } }
      server { listen 443 ssl; server_name api.goal.test;
        ssl_certificate /goal/tls.crt; ssl_certificate_key /goal/tls.key;
        location / { proxy_pass http://api:21114; proxy_set_header Host $host; proxy_set_header X-Forwarded-Proto https; } }
      server { listen 443 ssl; server_name idp.goal.test;
        ssl_certificate /goal/tls.crt; ssl_certificate_key /goal/tls.key;
        location / { proxy_pass http://keycloak:8080; proxy_set_header Host $host; proxy_set_header X-Forwarded-Host $host; proxy_set_header X-Forwarded-Proto https; } }
    }\n''')
    proxy_port = manifest['api_control_url'].rsplit(':', 1)[1]
    proxy = container('proxy', 'nginx:1.28.0', ['--network-alias', 'api.goal.test', '--network-alias', 'idp.goal.test',
        '-p', '127.0.0.1:' + proxy_port + ':80', '-v', str(root / 'nginx.conf') + ':/etc/nginx/nginx.conf:ro',
        '-v', str(root / 'tls.crt') + ':/goal/tls.crt:ro', '-v', str(root / 'tls.key') + ':/goal/tls.key:ro'])
    manifest['services']['proxy'] = proxy
    save()
    wait(lambda: api(manifest, '/health/ready'))
    result = api(manifest, '/api/admin/login', 'POST', {'username': 'goal-admin', 'password': private['admin_password']})
    token = result['access_token']
    private['admin_token'] = token
    account = api(manifest, '/api/admin/user/create', 'POST',
                  {'username': 'goal-user', 'password': private['account_password'], 'email': 'user@goal.test'}, token)
    private['account_id'] = account['id']
    issuer = 'https://idp.goal.test/realms/goal'
    wait(lambda: 'issuer' in run('docker', 'exec', service, 'curl', '--fail', '--silent', issuer + '/.well-known/openid-configuration'))
    api(manifest, '/api/admin/oauth/providers', 'POST', {'name': 'keycloak', 'kind': 'oidc', 'enabled': True,
        'client_id': 'goal-client', 'client_secret': private['oidc_secret'], 'issuer_url': issuer,
        'authorization_url': issuer + '/protocol/openid-connect/auth', 'token_url': issuer + '/protocol/openid-connect/token',
        'userinfo_url': issuer + '/protocol/openid-connect/userinfo', 'jwks_url': issuer + '/protocol/openid-connect/certs',
        'scopes': 'openid profile email'}, token)
    write(root / 'credentials.json', json.dumps(private, indent=2))
    key = run('docker', 'exec', service, 'cat', '/data/id_ed25519.pub')
    for letter in ['a', 'b']:
        home = prefix + '-home-' + letter
        manifest['volumes'].append(home)
        save()
        run('docker', 'volume', 'create', home)
        gui_port = port()
        client = container('client-' + letter, args.client_image, ['--init', '--hostname', 'Goal-' + letter.upper() + '-Laptop',
            '--cgroupns=private', '--cap-add', 'SYS_ADMIN', '--security-opt', 'apparmor=unconfined',
            '--shm-size', '512m', '-p', f'127.0.0.1:{gui_port}:6080', '-v', home + ':/home/tester',
            '-v', str(root / 'ca.crt') + ':/goal/ca.crt:ro', '-e', 'GOAL_ID_SERVER=hbbs:21116',
            '-e', 'GOAL_RELAY_SERVER=hbbr:21117', '-e', 'GOAL_SERVER_KEY=' + key])
        manifest['clients'][letter] = {'container': client, 'desktop': f'http://127.0.0.1:{gui_port}/vnc.html?autoconnect=true&resize=scale'}
        save()
        wait(lambda: urllib.request.urlopen(f'http://127.0.0.1:{gui_port}/vnc.html', timeout=2).status == 200)
        fixture = "from pathlib import Path\nimport os,hashlib,json\np=Path('/home/tester/transfers/source');p.mkdir(parents=True,exist_ok=True)\n(p/'随机文件.bin').write_bytes(os.urandom(1024*1024))\n(p/'空文件.txt').write_bytes(b'')\nprint(json.dumps({f.name:hashlib.sha256(f.read_bytes()).hexdigest() for f in p.iterdir()},ensure_ascii=False))"
        hashes = run('docker', 'exec', '--user', 'tester', '-i', client, 'python3', '-c', fixture)
        manifest['clients'][letter]['source_hashes'] = json.loads(hashes)
    manifest['checks']['provisioning'] = 'passed'
    manifest['image_ids'] = {image: run('docker', 'image', 'inspect', '--format', '{{.Id}}', image)
                             for image in [args.image, args.client_image, 'quay.io/keycloak/keycloak:26.4.0', 'nginx:1.28.0']}
    manifest['server_binary_hashes'] = run('docker', 'exec', service, 'sha256sum', '/usr/bin/hbbs', '/usr/bin/hbbr', '/usr/bin/rustdesk-api')
    save()
    print(json.dumps({'manifest': str(root / 'manifest.json'), 'desktops': manifest['clients'],
                      'api': 'https://api.goal.test', 'oidc_user': 'goal-oidc'}, ensure_ascii=False))


def collect(root, manifest):
    private = json.loads((root / 'credentials.json').read_text()) if (root / 'credentials.json').exists() else {}
    sensitive = [v for k,v in private.items() if any(word in k for word in ['password', 'secret', 'token'])]
    def redact(text):
        for value in sensitive:
            text = text.replace(value, '[REDACTED]')
        text = re.sub(r'eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+', '[REDACTED-JWT]', text)
        return re.sub(r'([?&](?:state|session_state|launch|code|nonce|code_challenge|id_token|access_token)=)[^&\s"<>]+', r'\1[REDACTED]', text)
    evidence = root / 'evidence'
    evidence.mkdir(exist_ok=True)
    for name in manifest['containers']:
        logs = run('docker', 'logs', name, check=False, include_stderr=True)
        write(evidence / (name + '.log'), redact(logs))
    for letter, client in manifest['clients'].items():
        log = run('docker', 'exec', client['container'], 'cat', '/home/tester/evidence/client.log', check=False)
        write(evidence / ('client-' + letter + '.log'), redact(log))
        log = run('docker', 'exec', client['container'], 'cat',
                  '/home/tester/.local/share/logs/RustDesk/rustdesk_rCURRENT.log', check=False)
        write(evidence / ('client-' + letter + '-rustdesk.log'), redact(log))
    print(str(evidence))


def verify_transfers(root, manifest, mode):
    evidence = root / 'evidence'
    evidence.mkdir(exist_ok=True)
    probe = "from pathlib import Path;import hashlib,json,sys;p=Path(sys.argv[1]);print(json.dumps({str(f.relative_to(p)):{'sha256':hashlib.sha256(f.read_bytes()).hexdigest(),'size':f.stat().st_size} for f in p.rglob('*') if f.is_file()},ensure_ascii=False))"
    results = {}
    for sender, receiver in [('a', 'b'), ('b', 'a')]:
        source = json.loads(run('docker', 'exec', manifest['clients'][sender]['container'],
                                'python3', '-c', probe, '/home/tester/transfers/source'))
        expected = manifest['clients'][sender]['source_hashes']
        if {name: value['sha256'] for name, value in source.items()} != expected:
            raise AssertionError('source fixture changed since provisioning')
        destination = json.loads(run('docker', 'exec', manifest['clients'][receiver]['container'],
                                     'python3', '-c', probe,
                                     f'/home/tester/transfers/{mode}-from-{sender}/source'))
        results[sender + '-to-' + receiver] = {'expected': source, 'actual': destination,
                                              'passed': source == destination}
    write(evidence / (mode + '-transfer-hashes.json'), json.dumps(results, indent=2, ensure_ascii=False))
    if not all(result['passed'] for result in results.values()):
        raise AssertionError('real client transfer bytes or file sizes differ')
    manifest['checks'][mode + '_file_transfer'] = 'passed'
    write(root / 'manifest.json', json.dumps(manifest, indent=2))
    print(mode + ' bidirectional file hashes and sizes passed')


def cleanup(root, manifest):
    collect(root, manifest)
    for name in reversed(manifest['containers']):
        run('docker', 'rm', '-f', name, check=False)
    for name in reversed(manifest['volumes']):
        run('docker', 'volume', 'rm', name, check=False)
    run('docker', 'network', 'rm', manifest['network'], check=False)
    remaining = run('docker', 'ps', '-a', '--filter', 'name=' + manifest['prefix'], '--format', '{{.Names}}')
    if remaining:
        raise RuntimeError('test containers remain after cleanup')
    volumes = set(run('docker', 'volume', 'ls', '--format', '{{.Name}}').splitlines())
    networks = set(run('docker', 'network', 'ls', '--format', '{{.Name}}').splitlines())
    if volumes.intersection(manifest['volumes']) or manifest['network'] in networks:
        raise RuntimeError('test volumes or network remain after cleanup')
    manifest['checks']['cleanup'] = 'passed'
    write(root / 'manifest.json', json.dumps(manifest, indent=2))
    for name in ['credentials.json', 'server.env', 'keycloak.env', 'realm.json', 'ca.key', 'tls.key']:
        (root / name).unlink(missing_ok=True)
    print('Exact test resources removed; evidence retained.')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('action', choices=['start', 'collect', 'verify-transfers', 'cleanup'])
    parser.add_argument('--mode', choices=['direct', 'relay'], default='direct')
    parser.add_argument('--directory', type=Path, required=True)
    parser.add_argument('--image', default='rustdesk-local/server:1.1.17-api-1.4.9')
    parser.add_argument('--client-image', default='rustdesk-local/desktop:1.4.9')
    args = parser.parse_args()
    root = args.directory.resolve()
    root.mkdir(parents=True, exist_ok=True)
    root.chmod(0o700)
    if args.action == 'start':
        start(args, root)
    else:
        manifest = json.loads((root / 'manifest.json').read_text())
        if args.action == 'verify-transfers':
            verify_transfers(root, manifest, args.mode)
        else:
            {'collect': collect, 'cleanup': cleanup}[args.action](root, manifest)


if __name__ == '__main__':
    main()
