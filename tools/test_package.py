"""Check Cargo's actual archive inventory against local development files."""
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

from report_paths import portable_text
from check_package import validate_inventory


class PackageTests(unittest.TestCase):
    def test_package_contains_only_consumer_build_inputs(self) -> None:
        project = Path(__file__).resolve().parent.parent
        required = {
            'LICENSE', 'README.md', 'THIRD_PARTY_NOTICES.md',
            'src/lib.rs', 'src/nested/runtime.rs', 'licenses/spacy-MIT.txt', 'licenses/thinc-MIT.txt',
        }
        excluded = {
            f'{directory}/{name}'
            for directory in ('assets/model', 'tools/node_modules/example', '.venv/example', 'unrelated')
            for name in ('LICENSE', 'README.md', 'SECURITY.md', 'THIRD_PARTY_NOTICES.md')
        } | {
            'assets/model/weights.safetensors', 'fixtures/unlisted.json', 'fixtures/hash.expected.json',
            'fixtures/token-match-v1.expected.json', 'fixtures/token-match-exhaustive-v1.expected.json',
            'fixtures/token-match-branching-v1.expected.json', 'fixtures/dependency-match-v1.expected.json',
            'fixtures/dependency-match-regressions-v1.expected.json', 'fixtures/README.md',
            'src/tests.rs', 'src/nested/tests.rs', 'examples/example.rs', 'tests/example.rs',
            'docs/guide.md', 'reference/README.md', 'reference/source-lock.json', 'SECURITY.md',
            'licenses/Unicode.txt', 'licenses/en_core_web_md-MIT.txt', 'licenses/en_core_web_md-SOURCES.txt',
        }
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            shutil.copyfile(project / 'crates/spars/Cargo.toml', root / 'Cargo.toml')
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

    def test_release_inventory_rejects_missing_inputs_and_extra_files(self) -> None:
        sources = {'src/lib.rs', 'src/nested/runtime.rs'}
        files = sources | {
            'Cargo.toml', 'Cargo.toml.orig', 'Cargo.lock', 'README.md', 'LICENSE',
            'THIRD_PARTY_NOTICES.md', 'licenses/spacy-MIT.txt', 'licenses/thinc-MIT.txt',
        }
        validate_inventory(files, sources)
        validate_inventory(files | {'.cargo_vcs_info.json'}, sources)
        for missing in sorted(files):
            with self.subTest(missing=missing), self.assertRaisesRegex(ValueError, 'missing='):
                validate_inventory(files - {missing}, sources)
        for extra in ('fixtures/expected.json', 'tests/example.rs', 'src/tests.rs', 'docs/guide.md', 'assets/weights'):
            with self.subTest(extra=extra), self.assertRaisesRegex(ValueError, 'unexpected='):
                validate_inventory(files | {extra}, sources)


if __name__ == '__main__':
    unittest.main()
