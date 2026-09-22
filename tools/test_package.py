"""Check Cargo's actual archive inventory against local development files."""
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

from report_paths import portable_text


class PackageTests(unittest.TestCase):
    def test_package_excludes_nested_readmes_notices_and_model_assets(self) -> None:
        project = Path(__file__).resolve().parent.parent
        required = {
            'LICENSE', 'README.md', 'SECURITY.md', 'THIRD_PARTY_NOTICES.md',
            'src/lib.rs', 'examples/example.rs', 'tests/example.rs', 'docs/guide.md',
            'licenses/notice.txt', 'fixtures/README.md', 'fixtures/hash.expected.json',
            'fixtures/token-match-v1.expected.json', 'fixtures/token-match-exhaustive-v1.expected.json',
            'fixtures/token-match-branching-v1.expected.json', 'fixtures/dependency-match-v1.expected.json',
            'fixtures/dependency-match-regressions-v1.expected.json',
            'reference/source-lock.json', 'reference/README.md',
        }
        excluded = {
            f'{directory}/{name}'
            for directory in ('assets/model', 'tools/node_modules/example', '.venv/example', 'unrelated')
            for name in ('LICENSE', 'README.md', 'SECURITY.md', 'THIRD_PARTY_NOTICES.md')
        } | {'assets/model/weights.safetensors', 'fixtures/unlisted.json'}
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            shutil.copyfile(project / 'Cargo.toml', root / 'Cargo.toml')
            shutil.copyfile(project / 'Cargo.lock', root / 'Cargo.lock')
            for relative in required | excluded:
                path = root / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text('')
            result = subprocess.run(
                ['cargo', 'package', '--list', '--offline'], cwd=root,
                text=True, capture_output=True, check=False,
            )
            self.assertEqual(result.returncode, 0, portable_text(result.stderr, root))
            inventory = set(result.stdout.splitlines())
            self.assertEqual(inventory & excluded, set(), 'Development files leaked into the package')
            self.assertEqual(required - inventory, set(), 'Required source or notices are missing')


if __name__ == '__main__':
    unittest.main()
