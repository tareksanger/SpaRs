"""Reference boundary, source provenance and frozen phrase fixture checks."""
import json
from itertools import takewhile
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
import spacy
from dataclasses import asdict
from unittest.mock import patch

from spacy.attrs import DEP, LEMMA, MORPH, POS, TAG
from spacy.symbols import IDS
from spacy.tokens import Doc as make_doc
from json_types import json_object, read_json
from phrase_match_reference import ROOT, AnnotatedToken, AnnotationFixture, build, build_annotations, build_edges, convert_matches, verify_sources

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

    def test_annotation_fixture_regenerates_and_agrees_with_an_independent_scan(self) -> None:
        fixture = build_annotations()
        self.assertEqual(asdict(fixture), json.loads((ROOT / 'fixtures/phrase-match-annotations-v1.expected.json').read_text()))
        self.assertIsInstance(fixture, AnnotationFixture)
        def key(attribute: str, token: AnnotatedToken) -> str:
            # spaCy's string for the stored key: no value is "", the empty analysis is "_".
            if attribute == 'NORM':
                return token.norm
            if attribute == 'MORPH':
                return '' if token.morph is None else token.morph or '_'
            value = {'LEMMA': token.lemma, 'POS': token.pos, 'TAG': token.tag, 'DEP': token.dep}[attribute]
            return value or ''
        attributes: set[str] = set()
        errors = 0
        partial = 0
        for case in fixture.cases:
            attributes.add(case.attribute)
            stored: dict[str, set[tuple[str, ...]]] = {}
            for op, state in zip(case.operations, case.states, strict=True):
                keys = [tuple(key(case.attribute, t) for t in pattern) for pattern in op.patterns if pattern]
                # spaCy registers the rule, then stores patterns until the first unannotated one raises.
                accepted = keys if case.attribute == 'NORM' else list(takewhile(lambda pattern: any(pattern), keys))
                rejected = len(accepted) < len(keys)
                self.assertEqual(state.error, 'ValueError' if rejected else None, case.id)
                if op.action == 'remove':
                    del stored[op.rule]
                else:
                    stored.setdefault(op.rule, set()).update(accepted)
                    errors += rejected
                    partial += rejected and bool(accepted)
                self.assertEqual(set(state.rules), set(stored), case.id)
                self.assertEqual(state.patterns, [sorted(list(p) for p in stored[rule]) for rule in state.rules], case.id)
                text = [key(case.attribute, t) for t in case.tokens]
                expected = {(rule, start, start + len(p)) for rule, patterns in stored.items() for p in patterns
                            for start in range(len(text) - len(p) + 1) if tuple(text[start:start + len(p)]) == p}
                self.assertEqual({(m.rule, m.start, m.end) for m in state.matches}, expected, case.id)
                self.assertEqual(len(state.matches), len(expected), case.id)
        self.assertEqual(attributes, {'NORM', 'LEMMA', 'POS', 'TAG', 'DEP', 'MORPH'})
        self.assertEqual((errors, partial), (45, 15))
        # Each annotation attribute matches something on model output and on missing values.
        for case in fixture.cases:
            if not case.id.startswith('unannotated-input') or case.attribute == 'NORM':
                self.assertTrue(any(state.matches for state in case.states), case.id)
            else:
                self.assertFalse(any(state.matches for state in case.states), case.id)

    def test_empty_annotation_strings_are_spacy_key_zero(self) -> None:
        nlp = spacy.blank('en')
        doc = make_doc(nlp.vocab, words=['a'])
        doc[0].lemma_ = ''
        doc[0].pos_ = ''
        doc[0].tag_ = ''
        doc[0].dep_ = ''
        doc[0].set_morph('')
        lemma, pos, tag, dep, empty = (int(value) for value in doc.to_array([LEMMA, POS, TAG, DEP, MORPH])[0])
        self.assertEqual((lemma, pos, tag, dep), (0, 0, 0, 0))
        self.assertNotEqual(empty, 0)
        self.assertEqual(nlp.vocab.strings[empty], '_')

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
