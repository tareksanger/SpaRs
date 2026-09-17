"""Check that the quality tools reject errors rather than merely running."""
import hashlib
import json
import tempfile
import unittest
from pathlib import Path

from check_docs import rust_examples, rustdoc_passed
from check_quality import check_fixtures, check_markdown


class QualityChecks(unittest.TestCase):
    def test_modified_fixture_fails(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root/'fixtures').mkdir()
            fixture = root/'fixtures/case.json'
            fixture.write_text('[]')
            (root/'fixtures/checksums.json').write_text(json.dumps({'fixtures/case.json':hashlib.sha256(fixture.read_bytes()).hexdigest()}))
            check_fixtures(root)
            fixture.write_text('[1]')
            with self.assertRaisesRegex(ValueError, 'Frozen fixture'):
                check_fixtures(root)
            fixture.unlink()
            with self.assertRaisesRegex(ValueError, 'inventory'):
                check_fixtures(root)

    def test_examples_must_execute(self):
        self.assertEqual(rust_examples('```rust\nassert_eq!(1, 1);\n```'), 1)
        for flag in ('no_run', 'ignore', 'compile_fail'):
            with self.assertRaises(ValueError):
                rust_examples(f'```rust,{flag}\nassert!(false);\n```')
            with self.assertRaises(ValueError):
                rust_examples(f'```{flag}\nassert!(false);\n```')

    def test_noncanonical_rust_fences_fail(self):
        for start, end in [('~~~rust', '~~~'), ('````rust', '````'), ('   ```rust', '   ```')]:
            with self.assertRaises(ValueError):
                rust_examples(f'{start}\nassert!(false);\n{end}')

    def test_only_executed_examples_pass(self):
        output = 'test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;'
        self.assertTrue(rustdoc_passed(0, output, 2))
        self.assertFalse(rustdoc_passed(1, output, 2))
        self.assertFalse(rustdoc_passed(0, output, 3))
        self.assertFalse(rustdoc_passed(0, output.replace('0 ignored', '1 ignored'), 2))
        self.assertFalse(rustdoc_passed(0, output.replace('0 filtered out', '1 filtered out'), 2))

    def test_markdown_keeps_structure_and_rejects_wrapping(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            path = root/'doc.md'
            path.write_text('# Title\n\nA paragraph.\n\n```rust\nlet a = 1;\nlet b = 2;\n```\n\n- One\n- Two\n')
            check_markdown(path, root)
            path.write_text('A hard-wrapped\nparagraph.\n')
            with self.assertRaisesRegex(ValueError, 'one source line'):
                check_markdown(path, root)

    def test_broken_link_fails(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            path = root/'doc.md'
            path.write_text('[missing](absent.md)\n')
            with self.assertRaisesRegex(ValueError, 'link'):
                check_markdown(path, root)

    def test_unclosed_fence_fails(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            path = root/'doc.md'
            path.write_text('```rust\nassert!(true);\n')
            with self.assertRaisesRegex(ValueError, 'unclosed'):
                check_markdown(path, root)


if __name__ == '__main__':
    unittest.main()
