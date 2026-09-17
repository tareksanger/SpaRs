"""Protect traversal fixture inputs and existing expected results."""
import json
from pathlib import Path
import tempfile
import unittest

from traversal_reference import TraversalFixture, Versions, input_cases, write_fixture


class TraversalReferenceTests(unittest.TestCase):
    def test_input_validation(self) -> None:
        case = {'id': 'one', 'text': 'word', 'tokens': [{'head': 0}]}
        corpus = {'versions': {'spacy': '3.8.14', 'thinc': '8.3.13'},
                  'model': 'en_core_web_md 3.8.0', 'cases': [case]}
        parsed = input_cases(json.dumps(corpus).encode())
        self.assertEqual([(item.id, item.text, item.heads) for item in parsed], [('one', 'word', [0])])
        with self.assertRaisesRegex(ValueError, 'unique'):
            input_cases(json.dumps({**corpus, 'cases': [case, case]}).encode())
        with self.assertRaisesRegex(ValueError, 'at least one'):
            input_cases(json.dumps({**corpus, 'cases': []}).encode())
        with self.assertRaisesRegex(ValueError, 'pinned'):
            input_cases(json.dumps({**corpus, 'model': 'other'}).encode())
        with self.assertRaisesRegex(ValueError, 'integer'):
            input_cases(json.dumps({**corpus, 'cases': [{**case, 'tokens': [{'head': True}]}]}).encode())

    def test_write_creates_directory_and_refuses_overwrite(self) -> None:
        fixture = TraversalFixture(Versions('3.8.14', '8.3.13'), 'en_core_web_md 3.8.0', 'digest', [])
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / 'new' / 'fixture.json'
            write_fixture(output, fixture)
            saved = output.read_bytes()
            with self.assertRaises(FileExistsError):
                write_fixture(output, fixture)
            self.assertEqual(output.read_bytes(), saved)


if __name__ == '__main__':
    unittest.main()
