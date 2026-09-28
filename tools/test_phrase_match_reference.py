"""Reference boundary, source provenance and frozen phrase fixture checks."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
import spacy
from dataclasses import asdict
from unittest.mock import patch

from spacy.symbols import IDS
from json_types import json_object, read_json
from phrase_match_reference import ROOT, build, build_edges, convert_matches, verify_sources

class PhraseReferenceTests(unittest.TestCase):
    def test_symbols_and_sources(self) -> None:
        versions, sources = verify_sources()
        self.assertEqual(versions, {'spacy': '3.8.14', 'preshed': '3.0.13'})
        self.assertEqual(len(sources), 5)
        self.assertEqual(json.loads((ROOT / 'crates/spars/src/phrase_matcher/symbols.json').read_text()), IDS)

    def test_reference_annotation_and_alias_probes(self) -> None:
        fixture = build(False)
        for probe in fixture.probes:
            if probe.attribute in ('ORTH', 'TEXT', 'orth'):
                self.assertIsNone(probe.error)
                self.assertTrue(probe.registered)
                self.assertEqual([(m.rule, m.start, m.end) for m in probe.matches], [('probe', 0, 1)])
            else:
                self.assertEqual(probe.error, 'ValueError')
                self.assertEqual(probe.registered, probe.attribute != 'INVALID')
                self.assertEqual(probe.matches, [])

    def test_edges_and_partial_error_state(self) -> None:
        fixture = build_edges()
        self.assertEqual(asdict(fixture), json.loads((ROOT / 'fixtures/phrase-match-edges-v1.expected.json').read_text()))
        self.assertEqual([(m.rule, m.start, m.end) for m in fixture.probes[0].matches], [('partial', 0, 1)])
        self.assertTrue(all(p.registered and p.error == 'ValueError' for p in fixture.probes))
        self.assertEqual(fixture.probes[1].matches, [])

    def test_rejects_malformed_match_values(self) -> None:
        vocab = spacy.blank('en').vocab
        for matches in [[(0, -1, 1)], [(0, 0, 2)], [(0, 1, 1)], [(True, 0, 1)]]:
            with self.subTest(matches=matches), self.assertRaises(ValueError):
                convert_matches(matches, vocab, 1)

    def test_generator_missing_directory_and_refuses_overwrite(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'absent' / 'reference.json'
            command = [sys.executable, str(ROOT / 'tools/phrase_match_reference.py'), str(path)]
            result = subprocess.run(command, capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(path.read_bytes(), (ROOT / 'fixtures/phrase-match-v1.expected.json').read_bytes())
            original = path.read_bytes()
            result = subprocess.run(command, capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('Refusing to overwrite', result.stderr)
            self.assertEqual(path.read_bytes(), original)

    def test_rejects_changed_secondary_source_and_version(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'reference').mkdir()
            (root / 'reference/source-lock.json').write_bytes((ROOT / 'reference/source-lock.json').read_bytes())
            extra = json_object(read_json(ROOT / 'reference/phrase-source-lock.json'))
            record = json_object(extra['preshed'])
            json_object(record['files'])['preshed/maps.pxd'] = 'bad'
            (root / 'reference/phrase-source-lock.json').write_text(json.dumps(extra))
            with patch('phrase_match_reference.ROOT', root), self.assertRaisesRegex(ValueError, 'maps.pxd'):
                verify_sources()
            record['version'] = '0.0.0'
            (root / 'reference/phrase-source-lock.json').write_text(json.dumps(extra))
            with patch('phrase_match_reference.ROOT', root), self.assertRaisesRegex(ValueError, 'version'):
                verify_sources()

    def test_rejects_unreviewed_source(self) -> None:
        with patch('phrase_match_reference.hashlib.sha256') as sha:
            sha.return_value.hexdigest.return_value = 'bad'
            with self.assertRaisesRegex(ValueError, 'Source mismatch'):
                verify_sources()

if __name__ == '__main__':
    unittest.main()
