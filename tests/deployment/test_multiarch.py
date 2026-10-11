"""Exercise release-image verification without Docker or filesystem cleanup."""
import contextlib
import copy
import io
import json
import os
from pathlib import Path
import struct
import subprocess
import sys
import textwrap
import unittest
from unittest import mock

import multiarch


IMAGE = 'ghcr.io/gloryhry/rustdesk-server@sha256:' + 'a' * 64
REVISION = 'b' * 40
BINARIES = ('hbbs', 'hbbr', 'rustdesk-api', 'rustdesk-utils')


class SavedLog(io.StringIO):
    def __init__(self, logs, name):
        super().__init__()
        self.logs = logs
        self.name = name

    def close(self):
        if not self.closed:
            self.logs[self.name] = self.getvalue()
        super().close()


class MultiarchTests(unittest.TestCase):
    def run_verification(self, platform='linux/amd64', mode='image', architecture=None,
                         header=None, exit_codes=None, help_text='Usage: service',
                         failure=None):
        expected_architecture, _, _, _, elf_class, elf_machine = multiarch.PLATFORMS[platform]
        if header is None:
            header = bytearray(20)
            header[:4] = b'\x7fELF'
            header[4] = elf_class
            header[5] = 1
            struct.pack_into('<H', header, 18, elf_machine)
        logs = {}
        commands = []
        reports = {}
        exit_codes = exit_codes or {}
        option = ['--image', IMAGE] if mode == 'image' else ['--builder', 'test-builder']
        argv = ['multiarch.py', '--platform', platform, '--log-dir', '/virtual/logs', *option]

        def output(command, **kwargs):
            commands.append(command)
            if command[0] == 'git':
                return REVISION + '\n'
            if command[:3] == ['docker', 'image', 'inspect']:
                return (architecture or expected_architecture) + '\n'
            if command[:2] == ['docker', 'create']:
                return 'test-container\n'
            raise AssertionError(f'Unexpected check_output: {command}')

        def run(command, **kwargs):
            commands.append(command)
            code = 0
            if command[-1] == '--help':
                binary = command[command.index('--entrypoint') + 1].rsplit('/', 1)[-1]
                kwargs['stdout'].write(help_text + '\n')
                code = exit_codes.get(binary, 1 if binary == 'rustdesk-utils' else 0)
            return subprocess.CompletedProcess(command, code)

        def open_path(path, mode='r', *args, **kwargs):
            if mode == 'rb':
                return io.BytesIO(header)
            if mode == 'w':
                return SavedLog(logs, path.name)
            raise AssertionError(f'Unexpected file mode: {mode}')

        def write_text(path, text, *args, **kwargs):
            reports[path.name] = text
            return len(text)

        with contextlib.ExitStack() as stack:
            stack.enter_context(mock.patch.object(sys, 'argv', argv))
            stack.enter_context(mock.patch.object(multiarch, 'source_hashes', return_value={'source': 'hash'}))
            stack.enter_context(mock.patch.object(multiarch.subprocess, 'check_output', side_effect=output))
            stack.enter_context(mock.patch.object(multiarch.subprocess, 'run', side_effect=run))
            temporary = stack.enter_context(mock.patch.object(multiarch.tempfile, 'TemporaryDirectory'))
            temporary.return_value.__enter__.return_value = '/virtual/binaries'
            stack.enter_context(mock.patch.object(Path, 'mkdir'))
            stack.enter_context(mock.patch.object(Path, 'unlink'))
            stack.enter_context(mock.patch.object(Path, 'open', autospec=True, side_effect=open_path))
            stack.enter_context(mock.patch.object(Path, 'read_text', autospec=True,
                side_effect=lambda path, *args, **kwargs: logs[path.name]))
            stack.enter_context(mock.patch.object(Path, 'write_text', autospec=True, side_effect=write_text))
            stack.enter_context(contextlib.redirect_stdout(io.StringIO()))
            if failure is None:
                multiarch.main()
            else:
                with self.assertRaises(failure):
                    multiarch.main()
                self.assertNotIn('result.json', reports)
        return commands, json.loads(reports['result.json']) if reports else None

    def test_image_mode_checks_all_four_platforms_without_building_or_removing_image(self):
        for platform, values in multiarch.PLATFORMS.items():
            with self.subTest(platform=platform):
                commands, report = self.run_verification(platform)
                self.assertEqual(report['platform'], platform)
                self.assertEqual(report['image'], IMAGE)
                self.assertEqual(report['head'], REVISION)
                self.assertEqual(report['checks'], ['runtime'])
                self.assertEqual(report['result'], 'passed')
                self.assertEqual(set(report['elf']['runtime']), set(BINARIES))
                for value in report['elf']['runtime'].values():
                    self.assertEqual(value, {'bits': 64 if values[4] == 2 else 32, 'machine': values[5]})
                self.assertFalse(any(command[:3] == ['docker', 'buildx', 'build'] for command in commands))
                self.assertFalse(any(command[:3] == ['docker', 'image', 'rm'] for command in commands))
                self.assertEqual(sum(command[-1] == '--help' for command in commands), 4)

    def test_builder_mode_preserves_existing_matrix_targets(self):
        expected = {
            'linux/amd64': ['runtime', 'supervisor', 'runtime'],
            'linux/arm64': ['runtime', 'supervisor'],
            'linux/arm/v7': ['runtime', 'supervisor', 'web-build'],
            'linux/386': ['supervisor', 'web-build'],
        }
        for platform, targets in expected.items():
            with self.subTest(platform=platform):
                commands, report = self.run_verification(platform, mode='builder')
                builds = [command for command in commands if command[:3] == ['docker', 'buildx', 'build']]
                self.assertEqual([command[command.index('--target') + 1] for command in builds], targets)
                self.assertNotIn('image', report)
                self.assertIn('supervisor', report['checks'])

    def test_rejects_wrong_image_architecture(self):
        self.run_verification(architecture='linux/arm64', failure=AssertionError)

    def test_rejects_wrong_or_truncated_elf_header(self):
        good = bytearray(20)
        good[:4] = b'\x7fELF'
        good[4:6] = bytes((2, 1))
        struct.pack_into('<H', good, 18, 62)
        bad_class = good.copy()
        bad_class[4] = 1
        bad_endian = good.copy()
        bad_endian[5] = 2
        bad_machine = good.copy()
        struct.pack_into('<H', bad_machine, 18, 183)
        for header in (b'', b'not an ELF header', bad_class, bad_endian, bad_machine):
            with self.subTest(header=header):
                self.run_verification(header=header, failure=AssertionError)

    def test_rejects_wrong_help_exit_codes(self):
        for binary, code in (('hbbs', 1), ('rustdesk-utils', 0)):
            with self.subTest(binary=binary):
                self.run_verification(exit_codes={binary: code}, failure=subprocess.CalledProcessError)

    def test_rejects_missing_help_output(self):
        self.run_verification(help_text='unexpected output', failure=AssertionError)

    def test_requires_exactly_one_mode(self):
        for options in ([], ['--builder', 'test', '--image', IMAGE]):
            with self.subTest(options=options), mock.patch.object(sys, 'argv',
                    ['multiarch.py', '--platform', 'linux/amd64', '--log-dir', '/virtual/logs', *options]), \
                    contextlib.redirect_stderr(io.StringIO()):
                with self.assertRaises(SystemExit) as result:
                    multiarch.main()
                self.assertEqual(result.exception.code, 2)


class ManifestPublicationTests(unittest.TestCase):
    def fixtures(self):
        image = IMAGE.split('@', 1)[0]
        records = []
        entries = []
        for number, platform in enumerate(multiarch.PLATFORMS, 1):
            digest = 'sha256:' + str(number) * 64
            records.append({'platform': platform, 'image': image, 'digest': digest, 'revision': REVISION})
            parts = platform.split('/')
            target = {'os': parts[0], 'architecture': parts[1]}
            if len(parts) == 3:
                target['variant'] = parts[2]
            entries.append({'platform': target, 'digest': digest})
        manifest = {'schemaVersion': 2, 'mediaType': 'application/vnd.oci.image.index.v1+json',
                    'manifests': entries}
        return records, manifest, [image + ':edge', image + ':sha-' + REVISION[:7]]

    def run_publication(self, records=None, candidate=None, tags=None, published=None,
                        published_digests=None, failure=None):
        default_records, default_candidate, default_tags = self.fixtures()
        records = default_records if records is None else records
        candidate = default_candidate if candidate is None else candidate
        tags = default_tags if tags is None else tags
        published = candidate if published is None else published
        published_digests = published_digests or ['sha256:' + 'c' * 64] * len(tags)
        workflow = (multiarch.ROOT / '.github/workflows/publish.yml').read_text()
        step = workflow.split('      - name: Merge and verify multi-platform tags\n', 1)[1]
        shell = step.split('        run: |\n', 1)[1].split('\n  release:', 1)[0]
        source = textwrap.dedent(shell).split("python3 - <<'PY'\n", 1)[1].rsplit('\nPY', 1)[0]
        files = [Path('/virtual/digests') / f'{number}.json' for number in range(len(records))]
        contents = {path: json.dumps(record) for path, record in zip(files, records)}
        commands = []
        environment = {'IMAGE_NAME': IMAGE.split('@', 1)[0], 'GITHUB_SHA': REVISION,
                       'DIGEST_DIR': '/virtual/digests', 'METADATA_JSON': json.dumps({'tags': tags})}

        def output(command, **kwargs):
            commands.append(command)
            if command[:4] == ['docker', 'buildx', 'imagetools', 'create'] and '--dry-run' in command:
                return json.dumps(candidate)
            if command[:4] == ['docker', 'buildx', 'imagetools', 'inspect']:
                if command[-1] == '--raw':
                    return json.dumps(published)
                if '--format' in command:
                    return published_digests[tags.index(command[4])] + '\n'
            raise AssertionError(f'Unexpected check_output: {command}')

        def run(command, **kwargs):
            commands.append(command)
            self.assertTrue(kwargs['check'])
            return subprocess.CompletedProcess(command, 0)

        with contextlib.ExitStack() as stack:
            stack.enter_context(mock.patch.dict(os.environ, environment))
            stack.enter_context(mock.patch.object(sys, 'path', list(sys.path)))
            stack.enter_context(mock.patch.object(Path, 'glob', return_value=files))
            stack.enter_context(mock.patch.object(Path, 'read_text', autospec=True,
                side_effect=lambda path, *args, **kwargs: contents[path]))
            stack.enter_context(mock.patch.object(subprocess, 'check_output', side_effect=output))
            stack.enter_context(mock.patch.object(subprocess, 'run', side_effect=run))
            stack.enter_context(contextlib.redirect_stdout(io.StringIO()))
            if failure is None:
                exec(compile(source, 'publish.yml:manifest', 'exec'), {})
            else:
                with self.assertRaises(failure):
                    exec(compile(source, 'publish.yml:manifest', 'exec'), {})
        return commands

    def assert_not_published(self, commands):
        self.assertFalse(any(command[:4] == ['docker', 'buildx', 'imagetools', 'create']
                            and '--dry-run' not in command for command in commands))

    def test_publishes_all_tags_only_after_candidate_verification(self):
        commands = self.run_publication()
        creates = [command for command in commands if command[:4] == ['docker', 'buildx', 'imagetools', 'create']]
        self.assertEqual(len(creates), 2)
        self.assertIn('--dry-run', creates[0])
        self.assertNotIn('--dry-run', creates[1])
        self.assertEqual(creates[1].count('--tag'), 2)
        self.assertEqual(sum('@sha256:' in argument for argument in creates[1]), 4)

    def test_rejects_missing_duplicate_or_stale_digest_records_before_publishing(self):
        original, _, _ = self.fixtures()
        duplicate_platform = copy.deepcopy(original)
        duplicate_platform[1]['platform'] = duplicate_platform[0]['platform']
        duplicate_digest = copy.deepcopy(original)
        duplicate_digest[1]['digest'] = duplicate_digest[0]['digest']
        stale = copy.deepcopy(original)
        stale[0]['revision'] = 'stale'
        wrong_image = copy.deepcopy(original)
        wrong_image[0]['image'] = 'ghcr.io/other/repository'
        bad_digest = copy.deepcopy(original)
        bad_digest[0]['digest'] = 'invalid'
        for records in (original[:-1], duplicate_platform, duplicate_digest, stale, wrong_image, bad_digest):
            with self.subTest(records=records):
                self.assert_not_published(self.run_publication(records=records, failure=SystemExit))

    def test_rejects_incomplete_duplicate_or_wrong_arm_variant_before_publishing(self):
        _, original, _ = self.fixtures()
        missing = copy.deepcopy(original)
        missing['manifests'].pop()
        duplicate = copy.deepcopy(original)
        duplicate['manifests'][2]['platform'] = duplicate['manifests'][1]['platform']
        wrong_arm = copy.deepcopy(original)
        wrong_arm['manifests'][2]['platform']['variant'] = 'v6'
        no_arm_variant = copy.deepcopy(original)
        no_arm_variant['manifests'][2]['platform'].pop('variant')
        attestation = copy.deepcopy(original)
        attestation['manifests'][0]['platform'] = {'os': 'unknown', 'architecture': 'unknown'}
        for candidate in (missing, duplicate, wrong_arm, no_arm_variant, attestation):
            with self.subTest(candidate=candidate):
                self.assert_not_published(self.run_publication(candidate=candidate, failure=SystemExit))

    def test_accepts_explicit_arm64_v8_variant(self):
        _, candidate, _ = self.fixtures()
        candidate['manifests'][1]['platform']['variant'] = 'v8'
        self.run_publication(candidate=candidate)

    def test_rejects_foreign_empty_or_duplicate_tags_before_publishing(self):
        for tags in ([], ['ghcr.io/other/repository:latest'], ['ghcr.io/gloryhry/rustdesk-server:edge'] * 2):
            with self.subTest(tags=tags):
                self.assert_not_published(self.run_publication(tags=tags, failure=SystemExit))

    def test_reports_failure_if_published_manifest_changes(self):
        _, published, _ = self.fixtures()
        published['manifests'][0]['digest'] = 'sha256:' + 'f' * 64
        self.run_publication(published=published, failure=SystemExit)

    def test_reports_failure_if_tags_resolve_to_different_index_digests(self):
        self.run_publication(published_digests=['sha256:' + 'c' * 64, 'sha256:' + 'd' * 64], failure=SystemExit)


if __name__ == '__main__':
    unittest.main()
