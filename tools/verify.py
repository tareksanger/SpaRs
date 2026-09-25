"""Run the acceptance checks and save their commands, results, and versions."""
import argparse
import hashlib
import json
import platform
import subprocess
import tempfile
from pathlib import Path
from typing import TypedDict
from json_types import ModelMetadata
from report_paths import portable_text


class Options(argparse.Namespace):
    model_exports_only: bool = False
    report: Path = Path("target/reports/verification.json")


class CommandResult(TypedDict):
    command: list[str]
    exit_code: int
    stdout: str
    stderr: str


class VerificationReport(TypedDict):
    model_reference_regeneration: bool
    platform: str
    machine: str
    rustc: str
    python: str
    reference_versions: dict[str, str]
    model: str
    commands: list[CommandResult]
    reexport_sha256: dict[str, str]
    fixture_sha256: dict[str, str]
    note: str


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def verification_commands(model_exports_only: bool) -> list[list[str]]:
    return [
        ["tools/node_modules/.bin/pyright", "--project", "pyrightconfig.json"],
        [".venv/bin/python", "tools/check_typing_policy.py"],
        ['.venv/bin/python', 'tools/check_quality.py'],
        ['.venv/bin/python', 'tools/check_reference.py'],
        ['.venv/bin/python', 'tools/check_models.py', *(['--exports-only'] if model_exports_only else [])],
        ['.venv/bin/python', '-m', 'unittest', 'discover', '-s', 'tools', '-p', 'test_*.py'],
        ['cargo', 'fmt', '--all', '--check'],
        ['cargo', 'fmt', '--manifest-path', 'consumer/Cargo.toml', '--check'],
        ['cargo', 'clippy', '--locked', '--offline', '--workspace', '--all-targets', '--', '-D', 'warnings'],
        ['npm', '--prefix', 'bindings/node', 'run', 'build'],
        ['npm', '--prefix', 'bindings/node', 'run', 'typecheck'],
        ['npm', '--prefix', 'bindings/node', 'test'],
        ['cargo', 'test', '--release', '--locked', '--offline', '--manifest-path', 'installer/Cargo.toml', '--', '--include-ignored'],
        ['cargo', 'build', '--release', '--locked', '--offline', '--manifest-path', 'installer/Cargo.toml'],
        ['cargo', 'test', '--release', '--locked', '--offline', '--', '--include-ignored'],
        ['.venv/bin/python', 'tools/check_docs.py'],
        ['cargo', 'build', '--release', '--locked', '--offline', '--manifest-path', 'consumer/Cargo.toml'],
        ['env', '-i', 'PATH=', 'consumer/target/release/native-consumer-check',
         'assets/en_core_web_md-3.8.0'],
        ['.venv/bin/python', 'tools/check_installer.py'],
        ['.venv/bin/python', 'tools/check_package.py'],
    ]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--report', type=Path, default=Path('target/reports/verification.json'))
    parser.add_argument('--model-exports-only', action='store_true',
                        help='Keep model export and native parity checks; regenerate sm/lg references separately on macOS ARM.')
    options = Options()
    parser.parse_args(namespace=options)
    report_path = options.report
    report_path.parent.mkdir(parents=True, exist_ok=True)
    results: list[CommandResult] = []

    def run(cmd: list[str]) -> None:
        result = subprocess.run(cmd, text=True, capture_output=True)
        results.append({'command':[portable_text(part, Path.cwd()) for part in cmd], 'exit_code':result.returncode,
                        'stdout':portable_text(result.stdout, Path.cwd()), 'stderr':portable_text(result.stderr, Path.cwd())})
        if result.returncode:
            failure = report_path.with_name(report_path.stem+'-failure.json')
            failure.write_text(json.dumps(results, indent=2)+'\n')
            print(result.stdout, result.stderr)
            raise SystemExit(f'Failed: {cmd}. See {failure}')
        print('PASS', ' '.join(results[-1]['command']), flush=True)

    commands = verification_commands(options.model_exports_only)
    for cmd in commands:
        run(cmd)
    Path('target').mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='reference-export-', dir='target') as tmp:
        run(['.venv/bin/python', 'tools/export.py', '--out', tmp])
        root = Path('assets/en_core_web_md-3.8.0')
        digests = {p.name:digest(p) for p in sorted(root.iterdir()) if p.is_file()}
        fresh = {p.name:digest(p) for p in sorted(Path(tmp).iterdir()) if p.is_file()}
        if digests != fresh:
            report_path.with_name(report_path.stem+'-failure.json').write_text(json.dumps(
                {'commands':results, 'error':'Model re-export differs', 'existing':digests,'fresh':fresh}, indent=2)+'\n')
            raise SystemExit('Model re-export differs; see failure report.')
    manifest = ModelMetadata.load(root/'manifest.json')
    report: VerificationReport = {'platform':platform.platform(),'machine':platform.machine(),
              'rustc':subprocess.check_output(['rustc','--version'],text=True).strip(),
              'python':platform.python_version(),'reference_versions':manifest.versions,
              'model_reference_regeneration': not options.model_exports_only,
              'model':manifest.model+' '+manifest.model_version, 'commands':results,
              'reexport_sha256':digests,
              'fixture_sha256':{str(p):digest(p) for p in sorted(Path('fixtures').glob('*.json'))},
              'note':'Local validation environment, not a performance benchmark.'}
    report_path.write_text(json.dumps(report,indent=2)+'\n')
    print('PASS byte-identical official re-export', flush=True)


if __name__ == '__main__':
    main()
