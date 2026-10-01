"""Build and check one release-image matrix entry without publishing anything."""
import argparse
import hashlib
import json
from pathlib import Path
import shlex
import struct
import subprocess
import tempfile
import uuid

ROOT = Path(__file__).resolve().parents[2]
PLATFORMS = {
    'linux/amd64': ('linux/amd64', 'x86_64', True, 'runtime', 2, 62),
    'linux/arm64': ('linux/arm64', 'aarch64', True, None, 2, 183),
    'linux/arm/v7': ('linux/arm', 'armhf', True, 'web-build', 1, 40),
    'linux/386': ('linux/386', 'i686', False, 'web-build', 1, 3),
}
SOURCE_FILES = ['docker/Dockerfile', 'docker-classic/Dockerfile', 'web/src/main.js',
    'Cargo.lock', 'web/package-lock.json', 'tests/deployment/multiarch.py',
    '.github/workflows/api-compatibility.yml', 'web/tests/address-book.spec.js']


def source_hashes():
    return {name: hashlib.sha256((ROOT / name).read_bytes()).hexdigest() for name in SOURCE_FILES}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--platform', choices=PLATFORMS, required=True)
    parser.add_argument('--builder', required=True)
    parser.add_argument('--log-dir', type=Path, required=True)
    args = parser.parse_args()
    architecture, s6_arch, runtime, classic, elf_class, elf_machine = PLATFORMS[args.platform]
    args.log_dir = args.log_dir.resolve()
    args.log_dir.mkdir(parents=True, exist_ok=True)
    (args.log_dir / 'result.json').unlink(missing_ok=True)
    source = source_hashes()
    source_revision = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()
    prefix = 'rustdesk-compat-' + uuid.uuid4().hex[:12]
    images = []
    checks = []
    checked_binaries = {}

    def run(name, command, expected_exit=0):
        with (args.log_dir / (name + '.log')).open('w') as log:
            log.write(shlex.join(command) + '\n')
            log.flush()
            print(shlex.join(command), flush=True)
            result = subprocess.run(command, cwd=ROOT, stdout=log, stderr=subprocess.STDOUT)
        if result.returncode != expected_exit:
            raise subprocess.CalledProcessError(result.returncode, command)

    def build(name, dockerfile, target):
        image = 'rustdesk-local/' + prefix + ':' + name
        images.append(image)
        run(name + '-build', ['docker', 'buildx', 'build', '--builder', args.builder,
            '--platform', args.platform, '--progress', 'plain', '--load',
            '--metadata-file', str(args.log_dir / (name + '-metadata.json')),
            '-f', dockerfile, '--target', target, '--build-arg', 'S6_ARCH=' + s6_arch,
            '-t', image, '.'])
        return image

    def check(name, image, web_only=False):
        if web_only:
            command = ['docker', 'run', '--rm', '--entrypoint', '/bin/sh', image, '-ec',
                'test "$(node --version)" = v24.9.0; test -s /source/web/dist/index.html; test -d /source/web/dist/assets']
        else:
            actual = subprocess.check_output(['docker', 'image', 'inspect', '--format',
                '{{.Os}}/{{.Architecture}}', image], text=True).strip()
            if actual != architecture:
                raise AssertionError(f'{name}: expected {architecture}, got {actual}')
            command = ['docker', 'run', '--rm', '--platform', args.platform,
                '--entrypoint', '/bin/sh', image, '-ec',
                'test -s /usr/share/rustdesk-api-web/index.html; test -d /usr/share/rustdesk-api-web/assets']
        run(name + '-check', command)
        if not web_only:
            # Inspect ELF headers too: installed QEMU can otherwise hide a wrong-architecture binary.
            container = subprocess.check_output(['docker', 'create', '--platform', args.platform,
                '--entrypoint', '/bin/sh', image, '-c', 'true'], text=True).strip()
            headers = {}
            try:
                with tempfile.TemporaryDirectory(prefix=prefix) as directory:
                    for binary in ['hbbs', 'hbbr', 'rustdesk-api', 'rustdesk-utils']:
                        destination = Path(directory) / binary
                        run(name + '-' + binary + '-copy', ['docker', 'cp', container + ':/usr/bin/' + binary, str(destination)])
                        with destination.open('rb') as source:
                            header = source.read(20)
                        if len(header) != 20 or header[:4] != b'\x7fELF' or header[4] != elf_class or header[5] != 1 or struct.unpack_from('<H', header, 18)[0] != elf_machine:
                            raise AssertionError(f'{name}: {binary} has the wrong ELF architecture')
                        headers[binary] = {'bits': 64 if elf_class == 2 else 32, 'machine': elf_machine}
            finally:
                subprocess.run(['docker', 'rm', container], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            for binary in ['hbbs', 'hbbr', 'rustdesk-api', 'rustdesk-utils']:
                # The existing utility CLI intentionally exits 1 after printing help.
                run(name + '-' + binary, ['docker', 'run', '--rm', '--platform', args.platform,
                    '--entrypoint', '/usr/bin/' + binary, image, '--help'],
                    expected_exit=1 if binary == 'rustdesk-utils' else 0)
                help_text = (args.log_dir / (name + '-' + binary + '.log')).read_text()
                if 'USAGE:' not in help_text and 'Usage:' not in help_text:
                    raise AssertionError(f'{name}: {binary} did not print help')
            checked_binaries[name] = headers
        checks.append(name)

    try:
        if runtime:
            check('runtime', build('runtime', 'docker/Dockerfile', 'runtime'))
        check('supervisor', build('supervisor', 'docker/Dockerfile', 'supervisor'))
        if classic:
            check('classic', build('classic', 'docker-classic/Dockerfile', classic), classic == 'web-build')
        if source_hashes() != source:
            raise AssertionError('source changed during matrix verification; rerun against the final source')
        report = {'platform': args.platform, 'checks': checks, 'elf': checked_binaries,
            'head': source_revision, 'source_sha256': source, 'result': 'passed'}
        (args.log_dir / 'result.json').write_text(json.dumps(report, indent=2) + '\n')
        print(json.dumps(report), flush=True)
    finally:
        for image in images:
            subprocess.run(['docker', 'image', 'rm', image], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


if __name__ == '__main__':
    main()
