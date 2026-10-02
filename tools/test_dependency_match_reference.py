"""Check reference conversion and frozen dependency matcher coverage."""
import subprocess
import sys
import tempfile
from typing import Callable
import unicodedata
from unittest.mock import patch
from pathlib import Path
import unittest

import dependency_match_reference
from dependency_match_reference import (
    FLAG_WORDS, LENGTH_WORDS, MORE_FLAG_ATTRIBUTES, MORE_FLAG_WORDS, NumberOperator, lexeme_flag, LOWER_VALUES, LOWER_WORDS, Compare, Constraint, IntegerMembership, Equals, Fixture, Flag, Link, Membership, Node, OPERATORS,
    Pattern, Versions, attribute_name, official, write_fixture, SET_DEPS, SET_LEMMAS, SET_MORPHS, SET_POS, SET_TAGS, SET_WORDS,
    set_constraints,
)
from json_types import json_array, json_int, json_object, json_string, parse_json


def expected_set_matches(constraints: list[Constraint], norms: list[str]) -> set[int]:
    """Tokens of the annotated set document that satisfy every constraint, computed
    without spaCy: a string value is a one-element set and morphology is the set of its
    individual Field=Value features, as in spaCy's set predicates."""
    def values(attribute: str, index: int) -> set[str | int]:
        word = SET_WORDS[index]
        if attribute == 'morphology':
            return {f'{key}={value}' for field in SET_MORPHS[index].split('|') if field
                    for key, joined in [field.split('=', 1)] for value in joined.split(',')}
        if attribute == 'length':
            return {len(word)}
        strings = {'text': word, 'lower': word.lower(), 'norm': norms[index], 'lemma': SET_LEMMAS[index],
                   'pos': SET_POS[index], 'tag': SET_TAGS[index], 'dep': SET_DEPS[index]}
        return {strings[attribute]}

    def normalize(entry: str) -> str:
        # A listed morphology entry equals one feature only if, after keeping the last
        # value of a repeated field and uppercasing the POS alias, one value remains.
        fields: dict[str, list[str]] = {}
        for field in entry.split('|'):
            key, _, joined = field.partition('=')
            if key.upper() == 'POS':
                key, joined = 'POS', joined.upper()
            fields[key] = joined.split(',')
        if len(fields) != 1:
            return entry
        [(key, values)] = fields.items()
        return f'{key}={values[0]}' if len(values) == 1 else entry

    def holds(constraint: Constraint, index: int) -> bool:
        actual = values(constraint.attribute, index)
        predicate = constraint.predicate
        if isinstance(predicate, Membership | IntegerMembership):
            expected: set[str | int] = set(predicate.values)
            if constraint.attribute == 'morphology' and isinstance(predicate, Membership):
                expected = {normalize(value) for value in predicate.values}
            kind = predicate.kind.removesuffix('_integers').removeprefix('morph_')
            if kind == 'in':
                return actual <= expected and len(actual) == 1
            if kind == 'is_subset':
                return actual <= expected
            if kind in ('is_superset', 'superset'):
                return actual >= expected
            if kind == 'intersects':
                return bool(actual & expected)
        if isinstance(predicate, Compare) and predicate.operator == '>':
            return all(value > predicate.value for value in actual if isinstance(value, int))
        raise ValueError(f'No independent rule for {constraint}')

    return {index for index in range(len(SET_WORDS)) if all(holds(constraint, index) for constraint in constraints)}


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
                generate.assert_called_once_with(False, True, False, False, False, False)
                write.assert_called_once()
                with patch('sys.argv', ['dependency_match_reference.py', '--lower', '--regressions', str(output)]):
                    with self.assertRaises(SystemExit):
                        dependency_match_reference.main()
                generate.reset_mock()
                with patch('sys.argv', ['dependency_match_reference.py', '--flags', str(output)]):
                    dependency_match_reference.main()
                generate.assert_called_once_with(False, False, True, False, False, False)
                generate.reset_mock()
                with patch('sys.argv', ['dependency_match_reference.py', '--length', str(output)]):
                    dependency_match_reference.main()
                generate.assert_called_once_with(False, False, False, True, False, False)
                generate.reset_mock()
                with patch('sys.argv', ['dependency_match_reference.py', '--more-flags', str(output)]):
                    dependency_match_reference.main()
                generate.assert_called_once_with(False, False, False, False, True, False)
                generate.reset_mock()
                with patch('sys.argv', ['dependency_match_reference.py', '--sets', str(output)]):
                    dependency_match_reference.main()
                generate.assert_called_once_with(False, False, False, False, False, True)
                with patch('sys.argv', ['dependency_match_reference.py', '--length', '--flags', str(output)]):
                    with self.assertRaises(SystemExit):
                        dependency_match_reference.main()

    def test_numeric_conditions_merge_into_one_official_dict(self) -> None:
        pattern = Pattern([Node('a', [Constraint('length', Compare('>=', 2)), Constraint('length', Compare('<', 4)),
                                      Constraint('length', IntegerMembership('not_in_integers', [3])),
                                      Constraint('lower', Equals('ab'))])])
        self.assertEqual(official(pattern), [{'RIGHT_ID': 'a', 'RIGHT_ATTRS': {
            'LOWER': 'ab', 'LENGTH': {'>=': 2, '<': 4, 'NOT_IN': [3]}}}])
        for duplicate in ([Compare('>', 1), Compare('>', 2)], [IntegerMembership('in_integers', [1]), IntegerMembership('in_integers', [2])]):
            with self.assertRaisesRegex(ValueError, 'distinct numeric operators'):
                official(Pattern([Node('a', [Constraint('length', item) for item in duplicate])]))
        with self.assertRaisesRegex(ValueError, 'distinct attributes'):
            official(Pattern([Node('a', [Constraint('lower', Equals('a')), Constraint('lower', Equals('b'))])]))

    def test_string_set_operators_merge_and_reject_mixed_kinds(self) -> None:
        pattern = Pattern([Node('a', [Constraint('morphology', Membership('is_superset', ['Number=Plur'])),
                                      Constraint('morphology', Membership('intersects', ['Tense=Past'])),
                                      Constraint('length', IntegerMembership('is_subset_integers', [3]))])])
        self.assertEqual(official(pattern), [{'RIGHT_ID': 'a', 'RIGHT_ATTRS': {
            'LENGTH': {'IS_SUBSET': [3]}, 'MORPH': {'IS_SUPERSET': ['Number=Plur'], 'INTERSECTS': ['Tense=Past']}}}])
        for mixed in ([Membership('is_subset', ['3']), Compare('>', 1)], [Compare('>', 1), Membership('is_subset', ['3'])],
                      [Equals('a'), Membership('in', ['a'])], [Membership('in', ['a']), Equals('a')]):
            with self.assertRaisesRegex(ValueError, 'distinct'):
                official(Pattern([Node('a', [Constraint('length', item) for item in mixed])]))
        with self.assertRaisesRegex(ValueError, 'distinct string operators'):
            official(Pattern([Node('a', [Constraint('lower', Membership('is_superset', ['a'])),
                                         Constraint('lower', Membership('is_superset', ['b']))])]))

    def test_frozen_length_suite_matches_python_len_per_token(self) -> None:
        path = Path(__file__).resolve().parent.parent / 'fixtures/dependency-match-length-v1.expected.json'
        fixture = json_object(parse_json(path.read_text()))
        cases = [json_object(value) for value in json_array(fixture['cases'])]
        self.assertEqual([case['id'] for case in cases], ['length-words', 'length-pipeline', 'length-empty'])
        self.assertEqual(sum(len(json_array(case['rules'])) for case in cases), 96)
        self.assertEqual(sum(len(json_array(case['expected'])) for case in cases), 321)
        self.assertEqual(cases[2]['expected'], [])
        self.assertEqual(cases[0]['text'], ' '.join(LENGTH_WORDS))
        found: dict[str, set[str]] = {}
        for value in json_array(cases[0]['expected']):
            match = json_object(value)
            tokens = json_array(match['tokens'])
            if len(tokens) == 1:
                found.setdefault(json_string(match['rule']), set()).add(LENGTH_WORDS[json_int(tokens[0])])
        # Independent invariant: LENGTH is Python len() of the token text.
        expected: dict[str, Callable[[int], bool]] = {
            'equals_3': lambda n: n == 3, 'equals_3_float': lambda n: n == 3, 'equals_0': lambda n: n == 0,
            'not_1': lambda n: n != 1, 'at_least_4': lambda n: n >= 4, 'at_most_1': lambda n: n <= 1,
            'over_2_5': lambda n: n > 2.5, 'under_2': lambda n: n < 2, 'under_huge': lambda n: True,
            'in_1_3': lambda n: n in (1, 3), 'in_negative': lambda n: n == 2, 'in_empty': lambda n: False,
            'in_large': lambda n: False, 'not_in_1_3': lambda n: n not in (1, 3), 'not_in_empty': lambda n: True,
            'between_2_4': lambda n: 2 <= n < 4, 'in_not_2': lambda n: n in (1, 3),
            'not_2_5': lambda n: n != 2.5, 'not_3_float': lambda n: n != 3.0,
            'equals_near_3': lambda n: n == 3.0000000000000004, 'at_least_near_3': lambda n: n >= 2.9999999999999996,
            'over_negative': lambda n: n > -1, 'in_duplicates': lambda n: n == 3,
            'in_and_not_in': lambda n: n in (1, 3),
            'equals_2_5': lambda n: n == 2.5, 'at_least_2_5': lambda n: n >= 2.5,
            'under_2_5': lambda n: n < 2.5, 'at_most_negative': lambda n: n <= -0.5,
        }
        for rule, accepts in expected.items():
            self.assertEqual(found.get(rule, set()), {word for word in LENGTH_WORDS if accepts(len(word))}, rule)
        self.assertEqual(found.get('short_not_abc', set()), set())
        self.assertEqual(found.get('long_member', set()), {'abcd'})
        # The parsed document must satisfy the same invariant token by token.
        pipeline = json_object(cases[1])
        text = json_string(pipeline['text']).encode()
        tokens = [json_object(token) for token in json_array(pipeline['tokens'])]
        words = [text[json_int(token['start']):json_int(token['end'])].decode() for token in tokens]
        matched: dict[str, set[int]] = {}
        for value in json_array(pipeline['expected']):
            match = json_object(value)
            indices = json_array(match['tokens'])
            if len(indices) == 1:
                matched.setdefault(json_string(match['rule']), set()).add(json_int(indices[0]))
        for rule, accepts in expected.items():
            self.assertEqual(matched.get(rule, set()), {i for i, word in enumerate(words) if accepts(len(word))}, rule)

    def test_flag_attributes_convert_to_official_boolean_values(self) -> None:
        self.assertEqual(attribute_name('like_num'), 'like_num')
        pattern = Pattern([Node('a', [Constraint('is_alpha', Flag(True)), Constraint('lower', Equals('hello'))])])
        self.assertEqual(official(pattern), [{'RIGHT_ID': 'a', 'RIGHT_ATTRS': {'IS_ALPHA': True, 'LOWER': 'hello'}}])
        with self.assertRaises(ValueError):
            attribute_name('is_oov')

    def test_frozen_flag_suite_counts_and_link(self) -> None:
        path = Path(__file__).resolve().parent.parent / 'fixtures/dependency-match-flags-v1.expected.json'
        fixture = json_object(parse_json(path.read_text()))
        cases = [json_object(value) for value in json_array(fixture['cases'])]
        self.assertEqual(sum(len(json_array(case['rules'])) for case in cases), 42)
        self.assertEqual(sum(len(json_array(case['expected'])) for case in cases), 376)
        self.assertEqual(cases[2]['expected'], [])
        self.assertEqual(cases[0]['text'], ' '.join(FLAG_WORDS))
        self.assertEqual(len(json_array(cases[0]['tokens'])), len(FLAG_WORDS))
        from spacy.lang.en.lex_attrs import like_num
        def is_punct(text: str) -> bool:
            return all(unicodedata.category(char).startswith('P') for char in text)
        checks: dict[str, Callable[[str], bool]] = {
            'is_alpha': str.isalpha, 'is_digit': str.isdigit, 'is_space': str.isspace,
            'is_punct': is_punct, 'like_num': like_num,
        }
        found: dict[int, set[str]] = {}
        for value in json_array(cases[0]['expected']):
            match = json_object(value)
            rule = json_string(match['rule'])
            if rule.endswith(('_true', '_false')):
                found.setdefault(json_int(json_array(match['tokens'])[0]), set()).add(rule)
        for index, text in enumerate(FLAG_WORDS):
            expected = {f'{name}_{str(bool(check(text))).lower()}' for name, check in checks.items()}
            self.assertEqual(found.get(index, set()), expected, text)
        words = json_object(cases[0])
        text = json_string(words['text']).encode()
        tokens = [json_object(token) for token in json_array(words['tokens'])]
        def token_text(index: int) -> str:
            return text[json_int(tokens[index]['start']):json_int(tokens[index]['end'])].decode()
        linked = [json_array(json_object(value)['tokens']) for value in json_array(cases[1]['expected'])
                  if json_object(value)['rule'] == 'word_with_number_child']
        pipeline = json_string(cases[1]['text']).encode()
        pipeline_tokens = [json_object(token) for token in json_array(cases[1]['tokens'])]
        def pipeline_text(index: int) -> str:
            return pipeline[json_int(pipeline_tokens[index]['start']):json_int(pipeline_tokens[index]['end'])].decode()
        pairs = [(pipeline_text(json_int(pair[0])), pipeline_text(json_int(pair[1]))) for pair in linked]
        self.assertTrue(pairs, 'the parsed document links a word to a numeric child')
        for head, child in pairs:
            self.assertTrue(head.isalpha() and like_num(child), (head, child))
        combined = {token_text(json_int(json_array(json_object(value)['tokens'])[0]))
                    for value in json_array(cases[0]['expected']) if json_object(value)['rule'] == 'number_word'}
        self.assertEqual(combined, {word for word in FLAG_WORDS if word.isalpha() and like_num(word)})


    def test_pinned_spacy_length_validation_matches_documented_contract(self) -> None:
        """Reference evidence for the rejection lists and the NaN/infinity difference."""
        import math
        import spacy
        from spacy.matcher import Matcher
        from spacy.tokens import Doc as make_doc
        nlp = spacy.blank('en')
        doc = make_doc(nlp.vocab, words=['a', 'abc'])
        def accepts(value: NumberOperator) -> bool:
            matcher = Matcher(nlp.vocab, validate=True)
            try:
                matcher.add('rule', [[{'LENGTH': value}]])
            except ValueError:
                return False
            matcher(doc)
            return True
        # Deliberately malformed values run in a child interpreter, outside the typed stub.
        script = (
            'import json, spacy\n'
            'from spacy.matcher import Matcher\n'
            'nlp = spacy.blank("en")\n'
            'rejected = []\n'
            'for value in [{"IN": [1.5]}, {"IN": [1.0]}, {"==": True}, {"IN": [True]}, {"IN": ["1"]}]:\n'
            '    try:\n'
            '        Matcher(nlp.vocab, validate=True).add("rule", [[{"LENGTH": value}]])\n'
            '    except ValueError:\n'
            '        rejected.append(True)\n'
            '    else:\n'
            '        rejected.append(False)\n'
            'print(json.dumps(rejected))\n'
        )
        output = subprocess.run([sys.executable, '-c', script], check=True, capture_output=True, text=True).stdout
        self.assertEqual(json_array(parse_json(output.strip().splitlines()[-1])), [True] * 5)
        # spaCy accepts non-finite comparison values; SpaRs rejects them (documented difference).
        for value in [NumberOperator({'>': math.nan}), NumberOperator({'<': math.inf}), NumberOperator({'>': -math.inf})]:
            self.assertTrue(accepts(value), value)


    def test_frozen_more_flag_suite_matches_spacy_lexical_attributes(self) -> None:
        """Each single-flag rule matches exactly the words whose lexeme has that flag."""
        import spacy
        nlp = spacy.blank('en')
        path = Path(__file__).resolve().parent.parent / 'fixtures/dependency-match-more-flags-v1.expected.json'
        fixture = json_object(parse_json(path.read_text()))
        cases = [json_object(value) for value in json_array(fixture['cases'])]
        self.assertEqual([case['id'] for case in cases], ['more-flag-words', 'more-flag-pipeline', 'more-flag-empty'])
        self.assertEqual(sum(len(json_array(case['rules'])) for case in cases), 90)
        self.assertEqual(sum(len(json_array(case['expected'])) for case in cases), 1209)
        self.assertEqual(cases[2]['expected'], [])
        self.assertEqual(cases[0]['text'], ' '.join(MORE_FLAG_WORDS))
        found: dict[str, set[int]] = {}
        for value in json_array(cases[0]['expected']):
            match = json_object(value)
            tokens = json_array(match['tokens'])
            if len(tokens) == 1:
                found.setdefault(json_string(match['rule']), set()).add(json_int(tokens[0]))
        # Independent of the matcher: the blank English vocabulary's lexeme flags.
        for attribute in MORE_FLAG_ATTRIBUTES:
            for value in (True, False):
                expected = {index for index, word in enumerate(MORE_FLAG_WORDS)
                            if lexeme_flag(nlp, word, attribute) == value}
                self.assertTrue(expected, f'{attribute} is {value} for no test word')
                self.assertEqual(found.get(f'{attribute}_{str(value).lower()}', set()), expected, (attribute, value))
        # Case and ASCII flags checked against Python's string methods, independent of spaCy.
        self.assertEqual(unicodedata.unidata_version, '15.0.0')
        methods: dict[str, Callable[[str], bool]] = {
            'is_lower': str.islower, 'is_upper': str.isupper, 'is_title': str.istitle, 'is_ascii': str.isascii,
        }
        for attribute, method in methods.items():
            self.assertEqual(found.get(f'{attribute}_true', set()),
                             {index for index, word in enumerate(MORE_FLAG_WORDS) if method(word)}, attribute)

    def test_frozen_set_suite_matches_one_element_and_feature_sets(self) -> None:
        """Every single-node set rule matches the tokens an independent set calculation selects."""
        path = Path(__file__).resolve().parent.parent / 'fixtures/dependency-match-sets-v1.expected.json'
        fixture = json_object(parse_json(path.read_text()))
        cases = [json_object(value) for value in json_array(fixture['cases'])]
        self.assertEqual([case['id'] for case in cases], ['set-annotated', 'set-pipeline', 'set-empty'])
        self.assertEqual(sum(len(json_array(case['rules'])) for case in cases), 546)
        self.assertEqual(sum(len(json_array(case['expected'])) for case in cases), 587)
        self.assertEqual(cases[2]['expected'], [])
        self.assertEqual(cases[0]['text'], ' '.join(SET_WORDS))
        found: dict[str, set[int]] = {}
        for value in json_array(cases[0]['expected']):
            match = json_object(value)
            tokens = json_array(match['tokens'])
            if len(tokens) == 1:
                found.setdefault(json_string(match['rule']), set()).add(json_int(tokens[0]))
        norms = [json_string(json_object(token)['norm']) for token in json_array(cases[0]['tokens'])]
        positive = 0
        for name, constraints in set_constraints():
            expected = expected_set_matches(constraints, norms)
            positive += bool(expected)
            self.assertEqual(found.get(name, set()), expected, name)
        # Both outcomes occur often enough to catch an operator that always or never matches.
        self.assertGreater(positive, 80)
        self.assertLess(positive, len(set_constraints()) - 50)

    def test_exported_url_pattern_is_the_one_like_url_uses(self) -> None:
        # spaCy's like_url uses spacy.lang.tokenizer_exceptions.URL_MATCH; SpaRs uses the
        # model's tokenizer URL pattern. Probed in a child interpreter, outside the stubs.
        script = (
            'import spacy\n'
            'from spacy.lang.tokenizer_exceptions import URL_MATCH\n'
            'nlp = spacy.load("en_core_web_md")\n'
            'print(nlp.tokenizer.url_match.__self__.pattern == URL_MATCH.__self__.pattern)\n'
        )
        output = subprocess.run([sys.executable, '-c', script], check=True, capture_output=True, text=True).stdout
        self.assertEqual(output.strip().splitlines()[-1], 'True')


if __name__ == '__main__':
    unittest.main()
