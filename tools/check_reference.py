"""Regenerate development, stage, and hash fixtures in isolation and compare bytes."""
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile


def main() -> None:
    root = Path(__file__).resolve().parent.parent
    expected = ['development.expected.json', 'stages.expected.json', 'hash.expected.json']
    with tempfile.TemporaryDirectory(prefix='spars-reference-check-') as temporary:
        work = Path(temporary)
        (work / 'fixtures').mkdir()
        shutil.copyfile(root / 'fixtures/development.json', work / 'fixtures/development.json')
        for script, arguments in [('fixtures.py', ['development']), ('stages.py', [])]:
            subprocess.run([sys.executable, str(root / 'tools' / script), *arguments], cwd=work, check=True)
        for name in expected:
            if (work / 'fixtures' / name).read_bytes() != (root / 'fixtures' / name).read_bytes():
                raise ValueError(f'Official reference output changed: {name}')
    print(f'PASS {len(expected)} regenerated reference fixtures are byte-identical')


if __name__ == '__main__':
    main()
