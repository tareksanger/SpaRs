"""Check reference conversion and frozen dependency matcher coverage."""
import tempfile
import unicodedata
from unittest.mock import patch
from pathlib import Path
import unittest

import dependency_match_reference
from dependency_match_reference import (
    LOWER_VALUES, LOWER_WORDS, Constraint, Equals, Fixture, Link, Membership, Node, OPERATORS,
    Pattern, Versions, attribute_name, official, write_fixture,
)
from json_types import json_array, json_int, json_object, json_string, parse_json


class DependencyReferenceTests(unittest.TestCase):
    def test_operator_conversion_preserves_link_and_attribute_semantics(self) -> None:
        pattern = Pattern([
            Node('verb', [Constraint('lemma', Equals('see'))]),
            Node('subject', [Constraint('morphology', Membership('morph_superset', ['Number=Sing']))], Link('verb', '>')),
        ])
        self.assertEqual(official(pattern), [
            {'RIGHT_ID': 'verb', 'RIGHT_ATTRS': {'LEMMA': 'see'}},
            {'RIGHT_ID': 'subject', 'RIGHT_ATTRS': {'MORPH': {'IS_SUPERSET': ['Number=Sing']}}, 'LEFT_ID': 'verb', 'REL_OP': '>'},
        ])

    def test_converter_rejects_repeated_attributes(self) -> None:
        pattern = Pattern([Node('a', [Constraint('text', Equals('a')), Constraint('text', Equals('b'))])])
        with self.assertRaisesRegex(ValueError, 'distinct attributes'):
            official(pattern)

    def test_frozen_regressions_preserve_morphology_hash_semantics(self) -> None:
        path = Path(__file__).resolve().parent.parent / 'fixtures/dependency-match-regressions-v1.expected.json'
        fixture = json_object(parse_json(path.read_text()))
        cases = [json_object(value) for value in json_array(fixture['cases'])]
        self.assertEqual(cases[0]['expected'], [
            {'rule': 'underscore_equals', 'tokens': [0]},
            {'rule': 'duplicate_field', 'tokens': [1]},
            {'rule': 'duplicate_value', 'tokens': [2]},
            {'rule': 'pos_lower', 'tokens': [3]},
            {'rule': 'pos_upper', 'tokens': [3]},
        ])
        self.assertEqual(cases[1]['expected'], [
            {'rule': 'selective_roots', 'tokens': [0]},
            {'rule': 'selective_roots', 'tokens': [1]},
        ])

    def test_unknown_attribute_fails(self) -> None:
        with self.assertRaises(ValueError):
            attribute_name('unknown')

    def test_writer_creates_directory_and_refuses_overwrite(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / 'new' / 'fixture.json'
            fixture = Fixture(Versions('3.8.14', '8.3.13'), 'en_core_web_md 3.8.0', 'test', [])
            write_fixture(output, fixture)
            before = output.read_bytes()
            with self.assertRaises(FileExistsError):
                write_fixture(output, fixture)
            self.assertEqual(before, output.read_bytes())

    def test_frozen_suite_covers_all_operators_and_crossing_dependencies(self) -> None:
        path = Path(__file__).resolve().parent.parent / 'fixtures/dependency-match-v1.expected.json'
        fixture = json_object(parse_json(path.read_text()))
        cases = [json_object(value) for value in json_array(fixture['cases'])]
        self.assertEqual({json_string(case['id']) for case in cases}, {'ordinary', 'sentences', 'unicode', 'empty', 'crossing', 'morphology'})
        found: set[str] = set()
        for case in cases:
            found.update(json_string(json_object(value)['rule']) for value in json_array(case['expected']))
        self.assertTrue(set(OPERATORS).issubset(found))
        empty = next(case for case in cases if case['id'] == 'empty')
        self.assertEqual(empty['expected'], [])
        morphology = next(case for case in cases if case['id'] == 'morphology')
        matches = [json_object(value) for value in json_array(morphology['expected'])]
        selected = {json_string(match['rule']): match['tokens'] for match in matches if json_string(match['rule']).startswith('morph_')}
        self.assertEqual(selected, {
            'morph_intersects': [0], 'morph_noncanonical_in': [0],
            'morph_single_superset': [0], 'morph_empty_in': [1],
            'morph_underscore_in': [1],
        })


    def test_lower_attribute_converts_to_official_name(self) -> None:
        self.assertEqual(attribute_name('lower'), 'lower')
        pattern = Pattern([Node('a', [Constraint('lower', Membership('in', ['the']))])])
        self.assertEqual(official(pattern), [{'RIGHT_ID': 'a', 'RIGHT_ATTRS': {'LOWER': {'IN': ['the']}}}])

    def test_frozen_lower_suite_matches_python_lowercase_and_links(self) -> None:
        path = Path(__file__).resolve().parent.parent / 'fixtures/dependency-match-lower-v1.expected.json'
        fixture = json_object(parse_json(path.read_text()))
        cases = [json_object(value) for value in json_array(fixture['cases'])]
        self.assertEqual([case['id'] for case in cases], ['lower-variants', 'lower-tree', 'lower-pipeline', 'lower-empty'])
        self.assertEqual(sum(len(json_array(case['rules'])) for case in cases), 160)
        self.assertEqual(sum(len(json_array(case['expected'])) for case in cases), 223)
        self.assertEqual(cases[3]['expected'], [])
        equals: dict[int, set[int]] = {}
        for value in json_array(cases[0]['expected']):
            match = json_object(value)
            rule = json_string(match['rule'])
            self.assertNotIn(rule, ('equals_1', 'in_uppercase'), 'pattern values must not be lowercased')
            if rule.startswith('equals_'):
                token = json_int(json_array(match['tokens'])[0])
                equals.setdefault(token, set()).add(int(rule.removeprefix('equals_')))
        self.assertEqual(unicodedata.unidata_version, '15.0.0')
        for index, text in enumerate(LOWER_WORDS):
            expected = {value for value, candidate in enumerate(LOWER_VALUES) if candidate == text.lower()}
            self.assertTrue(expected, text)
            self.assertEqual(equals.get(index, set()), expected, text)
        tree = [json_object(value) for value in json_array(cases[1]['expected'])]
        linked = [(match['rule'], match['tokens']) for match in tree if match['rule'] in ('child_the', 'subject_set')]
        self.assertEqual(linked, [('child_the', [1, 0]), ('subject_set', [2, 1]), ('subject_set', [2, 4])])

    def test_cli_passes_lower_mode_and_rejects_combined_modes(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / 'fixture.json'
            with patch.object(dependency_match_reference, 'generate') as generate, \
                    patch.object(dependency_match_reference, 'write_fixture') as write:
                with patch('sys.argv', ['dependency_match_reference.py', '--lower', str(output)]):
                    dependency_match_reference.main()
                generate.assert_called_once_with(False, True)
                write.assert_called_once()
                with patch('sys.argv', ['dependency_match_reference.py', '--lower', '--regressions', str(output)]):
                    with self.assertRaises(SystemExit):
                        dependency_match_reference.main()


if __name__ == '__main__':
    unittest.main()
