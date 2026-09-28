"""Check resource regeneration failures without rebuilding Unicode tables."""
from contextlib import ExitStack, redirect_stdout
from dataclasses import asdict
from io import StringIO
import json
from pathlib import Path
import tempfile
import subprocess
import unittest
from unittest.mock import patch

import phrase_lower_reference as reference
import check_reference


class PhraseLowerReferenceTests(unittest.TestCase):
    def test_failed_generator_removes_stale_reference_report(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            output = root / 'target/reports/reference-regeneration.json'
            output.parent.mkdir(parents=True)
            output.write_text('{"passed": true}')
            with patch.object(check_reference.subprocess, 'run', side_effect=subprocess.CalledProcessError(1, 'generator')):
                with self.assertRaises(subprocess.CalledProcessError):
                    check_reference.main(root)
            self.assertFalse(output.exists())

    def test_generation_and_check_reject_drift_without_overwriting(self) -> None:
        resource = reference.LowerResource('15.0.0', {'A': 'a'}, [[65, 90]], [])
        fixture = reference.LowerFixture(1, {'spacy': '3.8.14'}, {}, [])
        with tempfile.TemporaryDirectory() as temporary, ExitStack() as stack:
            root = Path(temporary)
            paths = [root / 'nested/lower.json', root / 'fixtures/lower.json']
            stack.enter_context(patch.object(reference, 'ROOT', root))
            stack.enter_context(patch.object(reference, 'RESOURCE', paths[0]))
            stack.enter_context(patch.object(reference, 'FIXTURE', paths[1]))
            stack.enter_context(patch.object(reference, 'build', return_value=(resource, fixture)))
            stack.enter_context(redirect_stdout(StringIO()))
            with patch('sys.argv', ['phrase_lower_reference.py']):
                reference.main()
                with self.assertRaisesRegex(ValueError, 'Refusing to overwrite'):
                    reference.main()
            expected = [json.dumps(asdict(item), ensure_ascii=False, indent=2) + '\n' for item in (resource, fixture)]
            self.assertEqual([path.read_text() for path in paths], expected)
            with patch('sys.argv', ['phrase_lower_reference.py', '--check']):
                reference.main()
                for index, path in enumerate(paths):
                    path.write_text('{}\n')
                    with self.assertRaisesRegex(ValueError, 'Regenerated content differs'):
                        reference.main()
                    self.assertEqual(path.read_text(), '{}\n', 'checking must never repair drift')
                    path.write_text(expected[index])
                paths[0].unlink()
                with self.assertRaises(FileNotFoundError):
                    reference.main()


if __name__ == '__main__':
    unittest.main()
