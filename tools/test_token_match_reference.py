"""Check the typed reference boundary and frozen repetition expectations."""
import tempfile
from typing import Callable
import unicodedata
from unittest.mock import patch
from pathlib import Path
import unittest

import token_match_reference
from dependency_match_reference import FLAG_WORDS, LOWER_VALUES, LOWER_WORDS, Constraint, Equals, Membership, Versions
from json_types import json_array, json_int, json_object, json_string, parse_json
from token_match_reference import Fixture, Item, Pattern, Range, Repetition, branching_rules, exhaustive_rules, flag, flag_rules, lower, lower_rules, official, word, write_fixture


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


    def test_lower_converter_uses_official_attribute_and_keeps_values_as_written(self) -> None:
        self.assertEqual(official(Pattern([lower('THE', Repetition('optional'))])), [{'LOWER': 'THE', 'OP': '?'}])
        names = [rule.name for rule in lower_rules()]
        self.assertEqual(len(names), len(set(names)))

    def test_frozen_lower_suite_matches_python_lowercase_per_token(self) -> None:
        path = Path(__file__).resolve().parent.parent / 'fixtures/token-match-lower-v1.expected.json'
        fixture = json_object(parse_json(path.read_text()))
        cases = [json_object(value) for value in json_array(fixture['cases'])]
        self.assertEqual([case['id'] for case in cases], ['lower-variants', 'lower-phrases', 'lower-pipeline', 'lower-empty'])
        self.assertEqual(sum(len(json_array(case['rules'])) for case in cases), 172)
        self.assertEqual(sum(len(json_array(case['expected'])) for case in cases), 280)
        self.assertEqual(cases[3]['expected'], [])
        matches = [json_object(value) for value in json_array(cases[0]['expected'])]
        equals: dict[int, set[int]] = {}
        for match in matches:
            rule = json_string(match['rule'])
            self.assertNotIn(rule, ('equals_1', 'uppercase_value', 'in_uppercase'), 'pattern values must not be lowercased')
            if rule.startswith('equals_'):
                equals.setdefault(json_int(match['start']), set()).add(int(rule.removeprefix('equals_')))
        # Independent invariant: official LOWER equality is Python str.lower() of the token text.
        self.assertEqual(unicodedata.unidata_version, '15.0.0')
        for index, text in enumerate(LOWER_WORDS):
            expected = {value for value, candidate in enumerate(LOWER_VALUES) if candidate == text.lower()}
            self.assertTrue(expected, text)
            self.assertEqual(equals.get(index, set()), expected, text)

    def test_cli_passes_lower_mode_and_rejects_combined_modes(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / 'fixture.json'
            with patch.object(token_match_reference, 'generate') as generate, \
                    patch.object(token_match_reference, 'write_fixture') as write:
                with patch('sys.argv', ['token_match_reference.py', '--lower', str(output)]):
                    token_match_reference.main()
                generate.assert_called_once_with(False, False, True, False, False)
                write.assert_called_once()
                with patch('sys.argv', ['token_match_reference.py', '--lower', '--exhaustive', str(output)]):
                    with self.assertRaises(SystemExit):
                        token_match_reference.main()
                generate.reset_mock()
                with patch('sys.argv', ['token_match_reference.py', '--flags', str(output)]):
                    token_match_reference.main()
                generate.assert_called_once_with(False, False, False, True, False)
                generate.reset_mock()
                with patch('sys.argv', ['token_match_reference.py', '--length', str(output)]):
                    token_match_reference.main()
                generate.assert_called_once_with(False, False, False, False, True)
                with patch('sys.argv', ['token_match_reference.py', '--flags', '--lower', str(output)]):
                    with self.assertRaises(SystemExit):
                        token_match_reference.main()

    def test_flag_converter_emits_strict_booleans(self) -> None:
        self.assertEqual(official(Pattern([flag('like_num', True, Repetition('one_or_more')), flag('is_punct', False)])),
                         [{'LIKE_NUM': True, 'OP': '+'}, {'IS_PUNCT': False}])
        names = [rule.name for rule in flag_rules()]
        self.assertEqual(len(names), len(set(names)))

    def test_frozen_flag_suite_matches_python_string_rules_per_token(self) -> None:
        from spacy.lang.en.lex_attrs import like_num
        path = Path(__file__).resolve().parent.parent / 'fixtures/token-match-flags-v1.expected.json'
        fixture = json_object(parse_json(path.read_text()))
        cases = [json_object(value) for value in json_array(fixture['cases'])]
        self.assertEqual([case['id'] for case in cases], ['flag-words', 'flag-pipeline', 'flag-empty'])
        self.assertEqual(sum(len(json_array(case['rules'])) for case in cases), 51)
        self.assertEqual(sum(len(json_array(case['expected'])) for case in cases), 442)
        self.assertEqual(cases[2]['expected'], [])
        self.assertEqual(cases[0]['text'], ' '.join(FLAG_WORDS))
        self.assertEqual(len(json_array(cases[0]['tokens'])), len(FLAG_WORDS))
        self.assertEqual(unicodedata.unidata_version, '15.0.0')
        found: dict[int, set[str]] = {}
        for value in json_array(cases[0]['expected']):
            match = json_object(value)
            rule = json_string(match['rule'])
            if rule.endswith(('_true', '_false')) and json_int(match['end']) - json_int(match['start']) == 1:
                found.setdefault(json_int(match['start']), set()).add(rule)
        def is_punct(text: str) -> bool:
            return all(unicodedata.category(char).startswith('P') for char in text)
        checks: dict[str, Callable[[str], bool]] = {
            'is_alpha': str.isalpha, 'is_digit': str.isdigit, 'is_space': str.isspace,
            'is_punct': is_punct, 'like_num': like_num,
        }
        for index, text in enumerate(FLAG_WORDS):
            expected = {f'{name}_{str(bool(check(text))).lower()}' for name, check in checks.items()}
            self.assertEqual(found.get(index, set()), expected, text)


    def test_frozen_length_suite_spans_satisfy_python_len(self) -> None:
        path = Path(__file__).resolve().parent.parent / 'fixtures/token-match-length-v1.expected.json'
        fixture = json_object(parse_json(path.read_text()))
        cases = [json_object(value) for value in json_array(fixture['cases'])]
        self.assertEqual([case['id'] for case in cases], ['length-words', 'length-pipeline', 'length-empty'])
        for case in cases[:2]:
            text = json_string(case['text']).encode()
            tokens = [json_object(token) for token in json_array(case['tokens'])]
            words = [text[json_int(token['start']):json_int(token['end'])].decode() for token in tokens]
            runs = [json_object(value) for value in json_array(case['expected']) if json_object(value)['rule'] == 'long_run']
            self.assertTrue(runs)
            for run in runs:
                self.assertTrue(all(len(word) >= 3 for word in words[json_int(run['start']):json_int(run['end'])]))


if __name__ == '__main__':
    unittest.main()
