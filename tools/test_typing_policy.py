"""Check that typing violations fail the policy checks."""
import unittest
import tempfile
from pathlib import Path

from check_typing_policy import check_source, source_paths


class TypingPolicyTests(unittest.TestCase):
    def test_nested_tools_are_checked(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            nested = root/'tools/package/helper.py'
            nested.parent.mkdir(parents=True)
            nested.write_text('from typing import Any')
            vendor = root/'tools/node_modules/vendor.py'
            vendor.parent.mkdir()
            vendor.write_text('from typing import Any')
            self.assertEqual(source_paths(root), [nested])
            with self.assertRaises(ValueError):
                check_source(nested.read_text(), str(nested))

    def test_concrete_annotations_pass(self) -> None:
        check_source('def size(words: list[str]) -> int:\n    return len(words)\n', 'example.py')

    def test_untyped_function_fails(self) -> None:
        for source in ('def size(words) -> int: return len(words)',
                       'def size(words: list[str]): return len(words)'):
            with self.assertRaisesRegex(ValueError, 'missing'):
                check_source(source, 'example.py')

    def test_escape_hatches_fail(self) -> None:
        for source in ('from typing import Any as Flexible',
                       'from typing import cast as trust',
                       'import typing\nx: typing.Any = 1',
                       'value = 1 # type: ignore',
                       '# pyright: reportUnknownVariableType=false'):
            with self.assertRaises(ValueError):
                check_source(source, 'example.py')


if __name__ == '__main__':
    unittest.main()
