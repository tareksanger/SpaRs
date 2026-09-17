"""Run the acceptance checks and save their commands, results, and versions."""
import argparse
import hashlib
import json
import platform
import subprocess
import tempfile
from pathlib import Path


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--report', type=Path, default=Path('reports/verification.json'))
    args = parser.parse_args()
    args.report.parent.mkdir(parents=True, exist_ok=True)
    results = []

    def run(cmd):
        result = subprocess.run(cmd, text=True, capture_output=True)
        results.append({'command':cmd, 'exit_code':result.returncode,
                        'stdout':result.stdout, 'stderr':result.stderr})
        if result.returncode:
            failure = args.report.with_name(args.report.stem+'-failure.json')
            failure.write_text(json.dumps(results, indent=2)+'\n')
            print(result.stdout, result.stderr)
            raise SystemExit(f'Failed: {cmd}. See {failure}')
        print('PASS', ' '.join(cmd), flush=True)

    commands = [
        ['.venv/bin/python', 'tools/check_quality.py'],
        ['.venv/bin/python', '-m', 'unittest', 'discover', '-s', 'tools', '-p', 'test_*.py'],
        ['cargo', 'fmt', '--check'],
        ['cargo', 'fmt', '--manifest-path', 'consumer/Cargo.toml', '--check'],
        ['cargo', 'clippy', '--offline', '--all-targets', '--', '-D', 'warnings'],
        ['cargo', 'test', '--release', '--offline', '--', '--include-ignored'],
        ['.venv/bin/python', 'tools/check_docs.py'],
        ['cargo', 'build', '--release', '--offline', '--manifest-path', 'consumer/Cargo.toml'],
        ['env', '-i', 'PATH=/nonexistent', 'consumer/target/release/native-consumer-check',
         str(Path('assets/en_core_web_md-3.8.0').resolve())],
        ['cargo', 'package', '--allow-dirty', '--offline'],
    ]
    for cmd in commands:
        run(cmd)
    with tempfile.TemporaryDirectory(prefix='spars-export-') as tmp:
        run(['.venv/bin/python', 'tools/export.py', '--out', tmp])
        root = Path('assets/en_core_web_md-3.8.0')
        digests = {p.name:digest(p) for p in sorted(root.iterdir()) if p.is_file()}
        fresh = {p.name:digest(p) for p in sorted(Path(tmp).iterdir()) if p.is_file()}
        if digests != fresh:
            args.report.with_name(args.report.stem+'-failure.json').write_text(json.dumps(
                {'commands':results, 'error':'Model re-export differs', 'existing':digests,'fresh':fresh}, indent=2)+'\n')
            raise SystemExit('Model re-export differs; see failure report.')
    manifest = json.loads((root/'manifest.json').read_text())
    report = {'platform':platform.platform(),'machine':platform.machine(),
              'rustc':subprocess.check_output(['rustc','--version'],text=True).strip(),
              'python':platform.python_version(),'reference_versions':manifest['versions'],
              'model':manifest['model']+' '+manifest['model_version'], 'commands':results,
              'reexport_sha256':digests,
              'fixture_sha256':{str(p):digest(p) for p in sorted(Path('fixtures').glob('*.json'))},
              'note':'Local validation environment, not a performance benchmark.'}
    args.report.write_text(json.dumps(report,indent=2)+'\n')
    print('PASS byte-identical official re-export', flush=True)


if __name__ == '__main__':
    main()
