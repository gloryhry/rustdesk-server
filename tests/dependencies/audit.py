"""Fail on selected RustSec advisories without hiding the other pending issues."""
import argparse
import json
import subprocess

parser = argparse.ArgumentParser()
parser.add_argument('--audit', default='cargo-audit')
parser.add_argument('advisories', nargs='+')
args = parser.parse_args()
result = subprocess.run([args.audit, 'audit', '--no-fetch', '--no-yanked', '--json'], capture_output=True, text=True)
report = json.loads(result.stdout)
if result.returncode not in (0, 1) or 'vulnerabilities' not in report:
    raise RuntimeError('audit could not run: ' + result.stderr)
remaining = {item['advisory']['id'] for item in report['vulnerabilities']['list']}
remaining.update(item['advisory']['id'] for item in report.get('warnings', {}).get('unsound', []))
failed = remaining.intersection(args.advisories)
if failed:
    raise SystemExit('Selected advisory regression failed: ' + ', '.join(sorted(failed)))
print(json.dumps({'checked': args.advisories, 'result': 'passed', 'other_pending_advisories': len(remaining)}))
