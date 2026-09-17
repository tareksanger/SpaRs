"""Run the Rust examples written in the README and developer guides."""
import json
import re
import subprocess
from pathlib import Path


def rust_examples(text):
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
        if info not in {'sh', 'bash', 'toml', 'json', 'text', 'python'} and info != 'rust':
            raise ValueError(f'Unsupported guide fence {info!r}; use plain rust or a documented non-Rust language.')
        if info == 'rust':
            if line != '```rust':
                raise ValueError('Runnable guide examples must use an unindented ```rust fence without flags.')
            count += 1
    if fence:
        raise ValueError('Unclosed guide code fence.')
    return count


def rustdoc_passed(returncode, stdout, count):
    passed = re.search(r'test result: ok\. (\d+) passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;', stdout)
    return returncode == 0 and passed is not None and int(passed[1]) == count


def main():
    root = Path(__file__).resolve().parent.parent
    subprocess.run(['cargo', 'build', '--release', '--offline', '--lib'], cwd=root, check=True)
    docs = [root/'README.md', *sorted((root/'docs').glob('*.md'))]
    results = []
    for path in docs:
        count = rust_examples(path.read_text())
        if not count:
            continue
        cmd = ['rustdoc', '--test', str(path), '--edition', '2021',
               '--extern', f'spars={root}/target/release/libspars.rlib',
               '-L', f'dependency={root}/target/release/deps',
               '--test-run-directory', str(root)]
        result = subprocess.run(cmd, cwd=root, text=True, capture_output=True)
        print(result.stdout, end='')
        print(result.stderr, end='')
        ok = rustdoc_passed(result.returncode, result.stdout, count)
        results.append({'document':str(path.relative_to(root)), 'examples':count,
                        'passed':ok,'stdout':result.stdout,'stderr':result.stderr})
    if not results:
        raise RuntimeError('No runnable Rust examples found.')
    (root/'reports').mkdir(exist_ok=True)
    (root/'reports/documentation.json').write_text(json.dumps(results,indent=2)+'\n')
    if not all(r['passed'] for r in results):
        raise SystemExit('Documentation examples failed; see reports/documentation.json')


if __name__ == '__main__':
    main()
