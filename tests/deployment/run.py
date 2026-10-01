"""Build and verify only disposable Compose, s6 and kind resources. Needs Docker/kind/kubectl."""
import argparse
import base64
import json
import os
from pathlib import Path
import secrets
import socket
import subprocess
import tempfile
import time
import urllib.request

ROOT = Path(__file__).resolve().parents[2]


def run(*args, capture=False, check=True, **kwargs):
    return subprocess.run(args, cwd=ROOT, check=check, text=True,
                          stdout=subprocess.PIPE if capture else None, **kwargs)


def eventually(check, timeout=180):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        try:
            if check():
                return
        except (OSError, ValueError, subprocess.CalledProcessError):
            pass
        time.sleep(1)
    raise AssertionError('deployment did not become ready')


def port():
    with socket.socket() as listener:
        listener.bind(('127.0.0.1', 0))
        return listener.getsockname()[1]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--skip-build', action='store_true')
    parser.add_argument('--kind', default='kind')
    parser.add_argument('--kubectl', default='kubectl')
    parser.add_argument('--image', default='rustdesk-local/server:1.1.17-api-1.4.9')
    args = parser.parse_args()
    image = args.image
    helper = 'python:3.12.8-slim-bookworm'
    project = 'rustdesk-check-' + secrets.token_hex(6)
    cluster_created = False
    containers = []
    volumes = []
    with tempfile.TemporaryDirectory(prefix=project) as temporary:
        tmp = Path(temporary)
        env = {
            'RUSTDESK_IMAGE': image, 'API_ENABLED': '1', 'API_BIND': '0.0.0.0',
            'API_PUBLIC_URL': 'https://api.example.com',
            'API_OAUTH_REDIRECT_URL': 'https://api.example.com/api/oidc/callback',
            'API_JWT_SECRET': secrets.token_urlsafe(48),
            'API_OAUTH_CONFIG_KEY': base64.b64encode(secrets.token_bytes(32)).decode(),
            'API_BOOTSTRAP_ADMIN_USERNAME': 'isolated-admin',
            'API_BOOTSTRAP_ADMIN_PASSWORD': secrets.token_urlsafe(32),
            'RUSTDESK_RELAY_SERVER': 'hbbr:21117',
            'API_HOST_PORT': str(port()), 'HBBS_NAT_PORT': '127.0.0.1:' + str(port()),
            'HBBS_PORT': '127.0.0.1:' + str(port()), 'HBBS_WS_PORT': '127.0.0.1:' + str(port()),
            'HBBR_PORT': '127.0.0.1:' + str(port()), 'HBBR_WS_PORT': '127.0.0.1:' + str(port()),
        }
        environment = tmp / 'compose.env'
        environment.write_text(''.join(key + '=' + value + '\n' for key, value in env.items()))
        environment.chmod(0o600)
        compose = ['docker', 'compose', '--env-file', str(environment), '-p', project]
        kubectl = [args.kubectl, '--kubeconfig', str(tmp / 'kubeconfig'), '-n', 'rustdesk-compat']

        def smoke(container, expected=None, hbbs='127.0.0.1:21116'):
            command = ['docker', 'run', '--rm', '--network', 'container:' + container,
                       '--env-file', str(environment), '-v', str(ROOT / 'tests/deployment') + ':/smoke:ro',
                       '-v', str(ROOT / 'tests/fixtures/rustdesk-1.4.9-address-book.json') + ':/requests.json:ro',
                       helper, 'python', '/smoke/smoke.py', '--fixtures', '/requests.json', '--hbbs', hbbs]
            if expected:
                command += ['--expected-guid', expected]
            return json.loads(run(*command, capture=True).stdout.strip())

        def healthy(container):
            return run('docker', 'inspect', '--format', '{{.State.Health.Status}}', container, capture=True).stdout.strip() == 'healthy'

        try:
            if not args.skip_build:
                for target, tag in [('runtime', image), ('supervisor', image + '-s6')]:
                    run('docker', 'build', '-f', 'docker/Dockerfile', '--target', target, '-t', tag, '.')
            run(*compose, 'up', '-d', '--no-build', '--wait', '--wait-timeout', '180')
            api = project + '-rustdesk-api-1'
            first = smoke(api, hbbs='hbbs:21116')
            run(*compose, 'restart', 'hbbs', 'hbbr', 'rustdesk-api')
            eventually(lambda: healthy(api))
            smoke(api, first['guid'])
            # Readiness must detect missing assets while liveness still responds.
            broken = project + '-broken'
            containers.append(broken)
            run('docker', 'run', '-d', '--name', broken, '--env-file', str(environment),
                '-e', 'API_WEB_ROOT=/missing', '-v', project + '_rustdesk-data:/data', image, 'rustdesk-api')
            def broken_ready():
                result = run('docker', 'exec', broken, 'sh', '-c',
                             'test "$(curl -s -o /dev/null -w "%{http_code}" http://127.0.0.1:21114/health/live)" = 200 && test "$(curl -s -o /dev/null -w "%{http_code}" http://127.0.0.1:21114/health/ready)" = 503', check=False, capture=True)
                return result.returncode == 0
            eventually(broken_ready)
            # Missing signing secrets cannot initialize a fresh volume.
            invalid = project + '-invalid'
            volumes.append(invalid)
            failed = run('docker', 'run', '--rm', '-e', 'API_ENABLED=1', '-v', invalid + ':/data', image, 'init', check=False, capture=True, stderr=subprocess.PIPE)
            assert failed.returncode != 0
            s6 = project + '-s6'
            containers.append(s6)
            volumes.append(s6)
            run('docker', 'run', '-d', '--name', s6, '--env-file', str(environment), '-v', s6 + ':/data', image + '-s6')
            eventually(lambda: healthy(s6))
            smoke(s6)
            # All six TCP endpoints must be listening, not just the API.
            run('docker', 'exec', s6, 'sh', '-c', 'for p in 21114 21115 21116 21117 21118 21119; do nc -z -w 2 127.0.0.1 "$p" || exit 1; done')

            # Core-only s6 must work without any API secrets or pre-existing keys.
            core = project + '-core-only'
            containers.append(core)
            volumes.append(core)
            run('docker', 'run', '-d', '--name', core, '-e', 'API_ENABLED=0', '-v', core + ':/data', image + '-s6')
            eventually(lambda: healthy(core))
            core_check = ['docker', 'exec', core, 'sh', '-c',
                          'nc -z -w 2 127.0.0.1 21116 && nc -z -w 2 127.0.0.1 21117 && ! nc -z -w 2 127.0.0.1 21114 && sha256sum /data/id_ed25519 /data/id_ed25519.pub']
            keys = run(*core_check, capture=True).stdout
            run('docker', 'restart', core)
            eventually(lambda: healthy(core))
            assert run(*core_check, capture=True).stdout == keys

            cluster_created = True
            run(args.kind, 'create', 'cluster', '--name', project, '--image', 'kindest/node:v1.34.0', '--kubeconfig', str(tmp / 'kubeconfig'), '--wait', '120s')
            run(args.kind, 'load', 'docker-image', image, '--name', project)
            run(*kubectl, 'create', 'namespace', 'rustdesk-compat')
            secret = tmp / 'secret.json'
            secret.write_text(json.dumps({'apiVersion': 'v1', 'kind': 'Secret', 'metadata': {'name': 'rustdesk-api-secret'}, 'stringData': {key: value for key, value in env.items() if key.startswith('API_')}}))
            secret.chmod(0o600)
            run(*kubectl, 'apply', '-f', str(secret))
            raw = run(*kubectl, 'create', '--dry-run=client', '-f', 'kubernetes/example.yaml', '-o', 'json', capture=True).stdout
            items = []
            decoder = json.JSONDecoder()
            while raw.strip():
                item, end = decoder.raw_decode(raw.lstrip())
                items.extend(item.get('items', [item]))
                raw = raw.lstrip()[end:]
            manifests = {'apiVersion': 'v1', 'kind': 'List', 'items': items}
            for item in manifests.get('items', [manifests]):
                if item.get('kind') == 'Deployment':
                    spec = item['spec']['template']['spec']
                    for container in spec.get('containers', []) + spec.get('initContainers', []):
                        container['image'] = image
            deployment = tmp / 'deployment.json'
            deployment.write_text(json.dumps(manifests))
            run(*kubectl, 'apply', '--dry-run=server', '-f', str(deployment))
            run(*kubectl, 'apply', '-f', str(deployment))
            scripts = run(*kubectl, 'create', 'configmap', 'smoke-scripts', '--from-file=smoke.py=tests/deployment/smoke.py', '--from-file=requests.json=tests/fixtures/rustdesk-1.4.9-address-book.json', '--dry-run=client', '-o', 'json', capture=True).stdout
            script_file = tmp / 'scripts.json'; script_file.write_text(scripts)
            run(*kubectl, 'apply', '-f', str(script_file))
            patch = tmp / 'patch.json'
            patch.write_text(json.dumps({'spec': {'template': {'spec': {
                'containers': [{'name': 'smoke', 'image': helper, 'command': ['sleep', '2147483647'], 'envFrom': [{'secretRef': {'name': 'rustdesk-api-secret'}}], 'volumeMounts': [{'name': 'smoke', 'mountPath': '/smoke', 'readOnly': True}]}],
                'volumes': [{'name': 'smoke', 'configMap': {'name': 'smoke-scripts'}}]
            }}}}))
            run(*kubectl, 'patch', 'deployment', 'rustdesk-server', '--patch-file', str(patch))
            run(*kubectl, 'rollout', 'status', 'deployment/rustdesk-server', '--timeout=180s')
            command = kubectl + ['exec', 'deployment/rustdesk-server', '-c', 'smoke', '--', 'python', '/smoke/smoke.py']
            kube_first = json.loads(run(*command, capture=True).stdout.strip())
            run(*kubectl, 'delete', 'pod', '-l', 'app=rustdesk', '--wait=true')
            run(*kubectl, 'rollout', 'status', 'deployment/rustdesk-server', '--timeout=180s')
            run(*command, '--expected-guid', kube_first['guid'])
            print(json.dumps({'compose': first['checks'], 's6': 'passed', 'kubernetes': 'passed', 'persistence': 'passed', 'negative_readiness': 'passed'}))
        finally:
            # Exact generated names only. Never prune unrelated containers or volumes.
            try:
                if cluster_created:
                    run(args.kind, 'delete', 'cluster', '--name', project, check=False)
            finally:
                for container in containers:
                    run('docker', 'rm', '-f', container, check=False)
                run(*compose, 'down', '-v', '--remove-orphans', check=False)
                for volume in volumes:
                    run('docker', 'volume', 'rm', volume, check=False)


if __name__ == '__main__':
    main()
