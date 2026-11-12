"""Run the Rust examples written in the README and developer guides."""
import json
import os
import re
import subprocess
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import TypedDict

from json_types import json_object, parse_json
from node_docs import examples as node_examples
from node_docs import run_example
from report_paths import portable_text

LIBRARY_CRATE = 'spars'


class DocumentationResult(TypedDict):
    document: str
    examples: int
    passed: bool
    stdout: str
    stderr: str


@dataclass(frozen=True, slots=True)
class RustLibrary:
    """Where Cargo wrote the library the guide examples link against."""
    rlib: Path
    dependency_directories: tuple[Path, ...]


def rust_library(build_messages: str) -> RustLibrary:
    """Locate the built library from the output of `cargo build --message-format=json`.

    Cargo's build directory layout is not a stable interface, so every path comes from the
    artifacts Cargo reports rather than from a fixed folder.
    """
    rlib: Path | None = None
    directories: dict[Path, None] = {}
    for line in build_messages.splitlines():
        if not line.startswith('{'):
            continue
        message = json_object(parse_json(line))
        if message.get('reason') != 'compiler-artifact':
            continue
        filenames = message.get('filenames')
        if not isinstance(filenames, list):
            raise ValueError('Cargo reported a compiler artifact without a list of file names.')
        paths = [Path(name) for name in filenames if isinstance(name, str)]
        for path in paths:
            directories[path.parent] = None
        if json_object(message.get('target')).get('name') == LIBRARY_CRATE:
            rlib = next((path for path in paths if path.suffix == '.rlib'), rlib)
    if rlib is None:
        raise RuntimeError(f'Cargo did not report an rlib for the {LIBRARY_CRATE} library.')
    return RustLibrary(rlib, tuple(directories))


def rust_examples(text: str) -> int:
    count = 0
    fence = None
    for line in text.splitlines():
        match = re.match(r"^ {0,3}(`{3,}|~{3,})(.*)$", line)
        if not match:
            continue
        marker, info = match.groups()
        if fence:
            if marker[0] == fence[0] and len(marker) >= len(fence) and not info.strip():
                fence = None
            continue
        fence = marker
        info = info.strip()
        if not info:
            raise ValueError('Guide code blocks need an explicit language.')
        if info not in {'sh', 'bash', 'toml', 'json', 'text', 'python', 'typescript'} and info != 'rust':
            raise ValueError(f'Unsupported guide fence {info!r}; use plain rust or a documented non-Rust language.')
        if info in {'rust', 'typescript'}:
            if line != '```' + info:
                raise ValueError('Runnable guide examples must use an unindented language fence without flags.')
        if info == 'rust':
            count += 1
    if fence:
        raise ValueError('Unclosed guide code fence.')
    return count


def rustdoc_passed(returncode: int, stdout: str, count: int) -> bool:
    passed = re.search(r'test result: ok\. (\d+) passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;', stdout)
    return returncode == 0 and passed is not None and int(passed[1]) == count


def check_documents(root: Path, store: Path) -> None:
    built = subprocess.run(['cargo', 'build', '--release', '--offline', '--lib', '--message-format=json'],
                           cwd=root, check=True, stdout=subprocess.PIPE, text=True)
    library = rust_library(built.stdout)
    subprocess.run(['cargo', 'build', '--release', '--offline', '--manifest-path',
                    'crates/spars-model/Cargo.toml', '--bin', 'spars'], cwd=root, check=True)
    installed = subprocess.run([
        'target/release/spars', 'download', 'en_core_web_sm', '--path', str(store),
        '--archive', 'assets/en_core_web_sm-3.8.0-py3-none-any.whl',
    ], cwd=root, capture_output=True, text=True)
    if installed.returncode:
        raise RuntimeError(portable_text(installed.stderr, root))
    os.environ['SPARS_MODEL_DIR'] = str(store)
    os.environ['SPARS_MODEL'] = str(root / 'assets/en_core_web_md-3.8.0')
    docs = [root/'README.md', *sorted((root/'docs').glob('*.md'))]
    results: list[DocumentationResult] = []
    for path in docs:
        count = rust_examples(path.read_text())
        for source in node_examples(path.read_text()):
            executed = run_example(root, source)
            print(executed.stdout, end='')
            print(executed.stderr, end='')
            results.append({'document': str(path.relative_to(root)), 'examples': 1,
                            'passed': executed.returncode == 0,
                            'stdout': portable_text(executed.stdout, root),
                            'stderr': portable_text(executed.stderr, root)})
        if not count:
            continue
        cmd = ['rustdoc', '--test', str(path.relative_to(root)), '--edition', '2021',
               '--extern', f'{LIBRARY_CRATE}={library.rlib}',
               *(flag for directory in library.dependency_directories
                 for flag in ('-L', f'dependency={directory}')),
               '--test-run-directory', '.']
        result = subprocess.run(cmd, cwd=root, text=True, capture_output=True)
        print(result.stdout, end='')
        print(result.stderr, end='')
        ok = rustdoc_passed(result.returncode, result.stdout, count)
        results.append({'document':str(path.relative_to(root)), 'examples':count,
                        'passed':ok,'stdout':portable_text(result.stdout, root),'stderr':portable_text(result.stderr, root)})
    if not results:
        raise RuntimeError('No runnable Rust examples found.')
    (root/'target/reports').mkdir(parents=True, exist_ok=True)
    (root/'target/reports/documentation.json').write_text(json.dumps(results,indent=2)+'\n')
    if not all(r['passed'] for r in results):
        raise SystemExit('Documentation examples failed; see target/reports/documentation.json')


def run_checks(root: Path) -> None:
    (root / 'target').mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='docs-models-', dir=root / 'target') as temporary:
        check_documents(root, Path(temporary))


def main() -> None:
    run_checks(Path(__file__).resolve().parent.parent)


if __name__ == '__main__':
    main()
