"""Check the typed reference boundary and frozen repetition expectations."""
import tempfile
from pathlib import Path
import unittest

from dependency_match_reference import Constraint, Equals, Membership, Versions
from json_types import json_array, json_object, json_string, parse_json
from token_match_reference import Fixture, Item, Pattern, Range, Repetition, branching_rules, exhaustive_rules, official, word, write_fixture


class TokenReferenceTests(unittest.TestCase):
    def test_branching_fixture_exercises_long_optional_suffixes(self) -> None:
        patterns = branching_rules()
        self.assertEqual(len(patterns), 8)
        self.assertEqual({len(rule.patterns[0].tokens) for rule in patterns}, {9, 10, 16, 17})
        path = Path(__file__).resolve().parent.parent / 'fixtures/token-match-branching-v1.expected.json'
        fixture = json_object(parse_json(path.read_text()))
        cases = [json_object(value) for value in json_array(fixture['cases'])]
        self.assertEqual(len(cases), 31)
        self.assertEqual(sum(len(json_array(case['rules'])) for case in cases), 248)
        self.assertEqual(sum(len(json_array(case['expected'])) for case in cases), 288)
        for case in cases:
            for value in json_array(case['expected']):
                self.assertNotIn('failing_tail', json_string(json_object(value)['rule']))

    def test_exhaustive_fixture_covers_all_short_quantifier_sequences(self) -> None:
        patterns = exhaustive_rules()
        self.assertEqual(len(patterns), 155)
        self.assertEqual(len({rule.name for rule in patterns}), 155)
        for length, count in [(1, 5), (2, 25), (3, 125)]:
            self.assertEqual(sum(len(rule.patterns[0].tokens) == length for rule in patterns), count)
        path = Path(__file__).resolve().parent.parent / 'fixtures/token-match-exhaustive-v1.expected.json'
        fixture = json_object(parse_json(path.read_text()))
        cases = [json_object(value) for value in json_array(fixture['cases'])]
        self.assertEqual(len(cases), 31)
        self.assertEqual(sum(len(json_array(case['rules'])) for case in cases), 4805)
        self.assertEqual(sum(len(json_array(case['expected'])) for case in cases), 6261)
        for length, count in [(0, 1), (1, 2), (2, 4), (3, 8), (4, 16)]:
            self.assertEqual(sum(len(json_array(case['tokens'])) == length for case in cases), count)

    def test_converter_preserves_predicates_and_repetition(self) -> None:
        pattern = Pattern([
            word('a'), word('b', Repetition('negated')), word('c', Range(2, None)),
            Item([Constraint('pos', Equals('NOUN')), Constraint('morphology', Membership('morph_superset', ['Number=Sing']))], Repetition('optional')),
        ])
        self.assertEqual(official(pattern), [
            {'ORTH': 'a'}, {'ORTH': 'b', 'OP': '!'}, {'ORTH': 'c', 'OP': '{2,}'},
            {'POS': 'NOUN', 'MORPH': {'IS_SUPERSET': ['Number=Sing']}, 'OP': '?'},
        ])

    def test_converter_rejects_duplicate_attributes_and_bad_ranges(self) -> None:
        with self.assertRaisesRegex(ValueError, 'distinct attributes'):
            official(Pattern([Item([Constraint('text', Equals('a')), Constraint('text', Equals('b'))])]))
        for repeat in [Range(-1, None), Range(2, 1)]:
            with self.assertRaisesRegex(ValueError, 'Invalid repetition range'):
                official(Pattern([word('a', repeat)]))

    def test_writer_creates_directories_and_refuses_overwrite(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / 'new' / 'fixture.json'
            fixture = Fixture(Versions('3.8.14', '8.3.13'), 'en_core_web_md 3.8.0', 'test', [])
            write_fixture(output, fixture)
            before = output.read_bytes()
            with self.assertRaises(FileExistsError):
                write_fixture(output, fixture)
            self.assertEqual(output.read_bytes(), before)

    def test_frozen_suite_has_unique_nonempty_matches_and_overlap_order(self) -> None:
        path = Path(__file__).resolve().parent.parent / 'fixtures/token-match-v1.expected.json'
        fixture = json_object(parse_json(path.read_text()))
        cases = [json_object(value) for value in json_array(fixture['cases'])]
        self.assertEqual(len(cases), 10)
        self.assertEqual(sum(len(json_array(case['expected'])) for case in cases), 704)
        for case in cases:
            expected = [json_object(value) for value in json_array(case['expected'])]
            keys = [f"{match['rule']}:{match['start']}:{match['end']}" for match in expected]
            self.assertEqual(len(keys), len(set(keys)))
            self.assertFalse(any(match['rule'] in ('range_0_0', 'never_negated_wildcard') for match in expected))
        empty = next(case for case in cases if case['id'] == 'empty')
        self.assertEqual(empty['expected'], [])
        repeated = next(case for case in cases if case['id'] == 'repetition-2')
        matches = [json_object(value) for value in json_array(repeated['expected'])]
        stars = [(match['start'], match['end']) for match in matches if json_string(match['rule']) == 'zero_or_more']
        self.assertEqual(stars, [(0, 1), (0, 2), (1, 2), (0, 3), (1, 3), (2, 3)])
        negated = [(match['start'], match['end']) for match in matches if match['rule'] == 'a_negated_b']
        self.assertEqual(negated, [(0, 2), (1, 3)])


if __name__ == '__main__':
    unittest.main()
