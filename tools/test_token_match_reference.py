"""Check the typed reference boundary and frozen repetition expectations."""
from dataclasses import dataclass
import tempfile
from typing import Callable, Literal, assert_never
import unicodedata
from unittest.mock import patch
from pathlib import Path
import unittest

import spacy
from spacy.matcher import Matcher
from spacy.tokens import Doc as make_doc

import token_match_reference
from dependency_match_reference import FLAG_WORDS, LOWER_VALUES, LOWER_WORDS, Constraint, Equals, Membership, Versions
from json_types import JsonValue, json_array, json_int, json_object, json_string, parse_json
from token_match_reference import (
    Case, Fixture, Greedy, Item, Pattern, Range, Repetition, branching_rules, exhaustive_rules, flag, flag_rules, greedy_interaction_rules,
    greedy_sequence_rules, lower, lower_rules, official, word, write_fixture,
)

GREEDY_FIXTURE = Path(__file__).resolve().parent.parent / 'fixtures/token-match-greedy-v1.expected.json'

type Span = tuple[int, int]
type NamedSpan = tuple[str, int, int]
type ComparedAttribute = Literal['text', 'pos']


@dataclass(frozen=True)
class RecordedToken:
    text: str
    pos: str

    def value(self, attribute: ComparedAttribute) -> str:
        match attribute:
            case 'text':
                return self.text
            case 'pos':
                return self.pos
            case _:
                assert_never(attribute)


@dataclass(frozen=True)
class RecordedItem:
    """One pattern item: `conditions` pairs a token attribute with the value it must equal."""
    conditions: list[tuple[ComparedAttribute, str]]
    minimum: int
    maximum: int | None
    negated: bool


@dataclass(frozen=True)
class RecordedRule:
    name: str
    patterns: list[list[RecordedItem]]
    greedy: Greedy | None


@dataclass(frozen=True)
class RecordedCase:
    id: str
    tokens: list[RecordedToken]
    rules: list[RecordedRule]
    expected: list[NamedSpan]
    unfiltered: list[NamedSpan]


def item_shape(item: Item) -> tuple[str | None, str]:
    """The token text a generated item requires, `None` for any token, and its repetition kind."""
    if not item.constraints:
        return None, item.repetition.kind
    [constraint] = item.constraints
    if constraint.attribute != 'text' or not isinstance(constraint.predicate, Equals):
        raise ValueError('The greedy sequence rules compare token text by equality only')
    return constraint.predicate.value, item.repetition.kind


def recorded_item(value: JsonValue) -> RecordedItem:
    item = json_object(value)
    conditions: list[tuple[ComparedAttribute, str]] = []
    for constraint in json_array(item['constraints']):
        attribute = json_string(json_object(constraint)['attribute'])
        predicate = json_object(json_object(constraint)['predicate'])
        if predicate['kind'] != 'equals' or (attribute != 'text' and attribute != 'pos'):
            raise ValueError(f'The greedy suite compares only text and pos by equality, not {attribute} by {predicate["kind"]}')
        conditions.append((attribute, json_string(predicate['value'])))
    repetition = json_object(item['repetition'])
    kind = json_string(repetition['kind'])
    if kind == 'range':
        maximum = repetition['max']
        return RecordedItem(conditions, json_int(repetition['min']), None if maximum is None else json_int(maximum), False)
    minimum, maximum, negated = {
        'once': (1, 1, False), 'optional': (0, 1, False), 'zero_or_more': (0, None, False),
        'one_or_more': (1, None, False), 'negated': (1, 1, True),
    }[kind]
    return RecordedItem(conditions, minimum, maximum, negated)


def recorded_spans(value: JsonValue) -> list[NamedSpan]:
    matches = [json_object(match) for match in json_array(value)]
    return [(json_string(match['rule']), json_int(match['start']), json_int(match['end'])) for match in matches]


def recorded_greedy_cases() -> list[RecordedCase]:
    cases: list[RecordedCase] = []
    for value in json_array(json_object(parse_json(GREEDY_FIXTURE.read_text()))['cases']):
        case = json_object(value)
        text = json_string(case['text']).encode()
        tokens = [json_object(token) for token in json_array(case['tokens'])]
        rules = [json_object(rule) for rule in json_array(case['rules'])]
        cases.append(RecordedCase(
            json_string(case['id']),
            [RecordedToken(text[json_int(token['start']):json_int(token['end'])].decode(), json_string(token['pos'])) for token in tokens],
            [RecordedRule(json_string(rule['name']),
                          [[recorded_item(item) for item in json_array(json_object(pattern)['tokens'])] for pattern in json_array(rule['patterns'])],
                          None if rule['greedy'] is None else Greedy(json_string(rule['greedy']))) for rule in rules],
            recorded_spans(case['expected']), recorded_spans(case['unfiltered']),
        ))
    return cases


def span_ends(items: list[RecordedItem], tokens: list[RecordedToken], index: int, position: int) -> set[int]:
    """Every position where `items[index:]` can stop after starting at `position`, read as a
    regular expression over tokens."""
    if index == len(items):
        return {position}
    item = items[index]
    ends: set[int] = set()
    count = 0
    while True:
        if count >= item.minimum:
            ends |= span_ends(items, tokens, index + 1, position + count)
        if count == item.maximum or position + count == len(tokens):
            return ends
        token = tokens[position + count]
        if all(token.value(attribute) == required for attribute, required in item.conditions) == item.negated:
            return ends
        count += 1


def pattern_spans(items: list[RecordedItem], tokens: list[RecordedToken]) -> set[Span]:
    return {(start, end) for start in range(len(tokens)) for end in span_ends(items, tokens, 0, start) if end > start}


def keep_without_overlap(spans: list[Span], greedy: Greedy) -> list[Span]:
    """The spans a greedy filter keeps, in the order it considers them."""
    match greedy:
        case Greedy.FIRST:
            ordered = sorted(spans, key=lambda span: (span[0], -span[1]))
        case Greedy.LONGEST:
            ordered = sorted(spans, key=lambda span: (span[0] - span[1], span[0]))
        case _:
            assert_never(greedy)
    kept: list[Span] = []
    for start, end in ordered:
        if all(end <= kept_start or kept_end <= start for kept_start, kept_end in kept):
            kept.append((start, end))
    return kept


def filtered_result(case: RecordedCase) -> list[NamedSpan]:
    """The result pinned spaCy returns for the case's registrations, derived from the recorded
    unfiltered matches: unfiltered rules first in recorded order, then each filtered rule in
    the order of its first recorded match."""
    # A later registration of a name replaces its filter.
    filters = {rule.name: rule.greedy for rule in case.rules}
    result = [match for match in case.unfiltered if filters[match[0]] is None]
    candidates: dict[str, list[Span]] = {}
    for name, start, end in case.unfiltered:
        if filters[name] is not None:
            candidates.setdefault(name, []).append((start, end))
    for name, spans in candidates.items():
        greedy = filters[name]
        assert greedy is not None
        result.extend((name, start, end) for start, end in keep_without_overlap(spans, greedy))
    return result


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
            fixture = Fixture(Versions('3.8.14', '8.3.13'), 'en_core_web_md 3.8.0', 'test', [Case('one', '', [], [], [], [], [], [])])
            with patch.object(token_match_reference, 'generate', return_value=fixture) as generate, \
                    patch.object(token_match_reference, 'write_fixture') as write, patch('builtins.print'):
                with patch('sys.argv', ['token_match_reference.py', '--lower', str(output)]):
                    token_match_reference.main()
                generate.assert_called_once_with(False, False, True, False, False, False, False, False)
                write.assert_called_once()
                with patch('sys.argv', ['token_match_reference.py', '--lower', '--exhaustive', str(output)]):
                    with self.assertRaises(SystemExit):
                        token_match_reference.main()
                generate.reset_mock()
                with patch('sys.argv', ['token_match_reference.py', '--flags', str(output)]):
                    token_match_reference.main()
                generate.assert_called_once_with(False, False, False, True, False, False, False, False)
                generate.reset_mock()
                with patch('sys.argv', ['token_match_reference.py', '--length', str(output)]):
                    token_match_reference.main()
                generate.assert_called_once_with(False, False, False, False, True, False, False, False)
                generate.reset_mock()
                with patch('sys.argv', ['token_match_reference.py', '--more-flags', str(output)]):
                    token_match_reference.main()
                generate.assert_called_once_with(False, False, False, False, False, True, False, False)
                generate.reset_mock()
                with patch('sys.argv', ['token_match_reference.py', '--sets', str(output)]):
                    token_match_reference.main()
                generate.assert_called_once_with(False, False, False, False, False, False, True, False)
                generate.reset_mock()
                with patch('sys.argv', ['token_match_reference.py', '--greedy', str(output)]):
                    token_match_reference.main()
                generate.assert_called_once_with(False, False, False, False, False, False, False, True)
                with patch('sys.argv', ['token_match_reference.py', '--flags', '--lower', str(output)]):
                    with self.assertRaises(SystemExit):
                        token_match_reference.main()
                with patch('sys.argv', ['token_match_reference.py', '--greedy', '--sets', str(output)]):
                    with self.assertRaises(SystemExit):
                        token_match_reference.main()
                # An existing output is refused before anything is generated.
                generate.reset_mock()
                output.write_text('frozen')
                with patch('sys.argv', ['token_match_reference.py', '--greedy', str(output)]):
                    with self.assertRaises(SystemExit):
                        token_match_reference.main()
                generate.assert_not_called()
                self.assertEqual(output.read_text(), 'frozen')

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


    def test_frozen_more_flag_suite_single_items_match_the_word_flags(self) -> None:
        import spacy
        from dependency_match_reference import MORE_FLAG_ATTRIBUTES, MORE_FLAG_WORDS, lexeme_flag
        nlp = spacy.blank('en')
        path = Path(__file__).resolve().parent.parent / 'fixtures/token-match-more-flags-v1.expected.json'
        fixture = json_object(parse_json(path.read_text()))
        case = json_object(json_array(fixture['cases'])[0])
        self.assertEqual(case['text'], ' '.join(MORE_FLAG_WORDS))
        found: dict[str, set[int]] = {}
        for value in json_array(case['expected']):
            match = json_object(value)
            if json_int(match['end']) - json_int(match['start']) == 1:
                found.setdefault(json_string(match['rule']), set()).add(json_int(match['start']))
        for attribute in MORE_FLAG_ATTRIBUTES:
            for flag_value in (True, False):
                expected = {i for i, word in enumerate(MORE_FLAG_WORDS) if lexeme_flag(nlp, word, attribute) == flag_value}
                self.assertEqual(found.get(f'{attribute}_{str(flag_value).lower()}', set()), expected, (attribute, flag_value))

    def test_frozen_set_suite_matches_one_element_and_feature_sets(self) -> None:
        """Every single-item set rule matches the tokens an independent set calculation selects."""
        from dependency_match_reference import set_constraints
        path = Path(__file__).resolve().parent.parent / 'fixtures/token-match-sets-v1.expected.json'
        fixture = json_object(parse_json(path.read_text()))
        cases = [json_object(value) for value in json_array(fixture['cases'])]
        self.assertEqual([case['id'] for case in cases], ['set-annotated', 'set-pipeline', 'set-empty'])
        self.assertEqual(sum(len(json_array(case['rules'])) for case in cases), 558)
        self.assertEqual(sum(len(json_array(case['expected'])) for case in cases), 653)
        self.assertEqual(cases[2]['expected'], [])
        from dependency_match_reference import SET_WORDS
        self.assertEqual(cases[0]['text'], ' '.join(SET_WORDS))
        found: dict[str, set[int]] = {}
        for value in json_array(cases[0]['expected']):
            match = json_object(value)
            if json_int(match['end']) - json_int(match['start']) == 1:
                found.setdefault(json_string(match['rule']), set()).add(json_int(match['start']))
        norms = [json_string(json_object(token)['norm']) for token in json_array(cases[0]['tokens'])]
        from test_dependency_match_reference import expected_set_matches
        positive = 0
        for name, constraints in set_constraints():
            expected = expected_set_matches(constraints, norms)
            positive += bool(expected)
            self.assertEqual(found.get(name, set()), expected, name)
        self.assertGreater(positive, 80)
        self.assertLess(positive, len(set_constraints()) - 50)

    def test_greedy_rules_register_every_operator_pair_under_both_filters(self) -> None:
        sequences = greedy_sequence_rules()
        self.assertEqual(len({rule.name for rule in sequences}), 120)
        kinds = ['once', 'optional', 'zero_or_more', 'one_or_more', 'negated']
        intended: set[tuple[tuple[str | None, str], ...]] = {(('a', first),) for first in kinds}
        intended |= {(('a', first), (text, second)) for first in kinds for text in ('a', 'b') for second in kinds}
        intended |= {(('a', 'once'), (None, 'once'), ('b', third)) for third in kinds}
        for greedy in Greedy:
            shapes = [tuple(item_shape(item) for item in rule.patterns[0].tokens) for rule in sequences if rule.greedy is greedy]
            self.assertEqual(len(shapes), 60)
            self.assertEqual(set(shapes), intended)
        interactions = greedy_interaction_rules()
        names = [rule.name for rule in interactions]
        self.assertEqual({name for name in names if names.count(name) > 1},
                         {'filter_then_none', 'none_then_filter', 'first_then_longest', 'longest_then_first',
                          'empty_sets_filter', 'empty_clears_filter', 'three_registrations'})
        self.assertEqual({rule.greedy for rule in interactions}, {None, *Greedy})

    def test_generator_refuses_a_runtime_other_than_the_pinned_one(self) -> None:
        with patch.object(token_match_reference.spacy, '__version__', '3.8.13'), \
                patch.object(token_match_reference.spacy, 'load') as load:
            with self.assertRaises(ValueError):
                token_match_reference.generate(greedy=True)
            load.assert_not_called()

    def test_frozen_greedy_suite_records_both_results_for_every_case(self) -> None:
        cases = recorded_greedy_cases()
        self.assertEqual(len(cases), 40)
        self.assertEqual(sum(len(case.tokens) for case in cases), 145)
        self.assertEqual(sum(len(case.rules) for case in cases), 4242)
        self.assertEqual(sum(len(case.expected) for case in cases), 4357)
        self.assertEqual(sum(len(case.unfiltered) for case in cases), 8361)
        for case in cases:
            self.assertEqual(len(case.unfiltered), len(set(case.unfiltered)), case.id)
            self.assertTrue(all(start < end for _, start, end in case.unfiltered), case.id)
            self.assertLessEqual(set(case.expected), set(case.unfiltered), case.id)
        for case_id in ('greedy-binary-empty', 'greedy-rules-empty'):
            empty = next(case for case in cases if case.id == case_id)
            self.assertEqual((empty.expected, empty.unfiltered), ([], []))

    def test_frozen_greedy_unfiltered_matches_are_every_span_the_patterns_allow(self) -> None:
        """Independent of spaCy: each rule's unfiltered spans are those its patterns match when
        read as regular expressions over the case's tokens."""
        for case in recorded_greedy_cases():
            allowed: dict[str, set[Span]] = {}
            for rule in case.rules:
                for pattern in rule.patterns:
                    allowed.setdefault(rule.name, set()).update(pattern_spans(pattern, case.tokens))
            found: dict[str, set[Span]] = {name: set() for name in allowed}
            for name, start, end in case.unfiltered:
                found[name].add((start, end))
            self.assertEqual(found, allowed, case.id)

    def test_frozen_greedy_results_keep_non_overlapping_spans_in_filter_order(self) -> None:
        """Independent of spaCy: the filtered result follows from the unfiltered matches."""
        for case in recorded_greedy_cases():
            self.assertEqual(case.expected, filtered_result(case), case.id)

    def test_frozen_greedy_suite_shows_each_filter_behaviour(self) -> None:
        cases = {case.id: case for case in recorded_greedy_cases()}

        def spans(case_id: str, rule: str) -> list[Span]:
            return [(start, end) for name, start, end in cases[case_id].expected if name == rule]

        # Result order is the order in which a filter accepts spans, not position order.
        self.assertEqual(spans('greedy-rules-runs', 'run_longest'), [(3, 6), (0, 2)])
        self.assertEqual(spans('greedy-rules-runs', 'run_first'), [(0, 2), (3, 6)])
        # The two filters can keep different spans.
        self.assertEqual(spans('greedy-rules-late-long', 'early_short_late_long_first'), [(0, 2)])
        self.assertEqual(spans('greedy-rules-late-long', 'early_short_late_long_longest'), [(1, 5)])
        self.assertEqual(spans('greedy-rules-chain', 'chain_first'), [(0, 2)])
        self.assertEqual(spans('greedy-rules-chain', 'chain_longest'), [(1, 4)])
        # Of equally long overlapping spans, the earlier one is kept.
        self.assertEqual(spans('greedy-rules-ties', 'pair_longest'), [(0, 2), (2, 4)])
        # Another rule name keeps the same spans.
        self.assertEqual(spans('greedy-rules-runs', 'run_longest_again'), spans('greedy-rules-runs', 'run_longest'))
        # The last registration of a name decides its filter for all its patterns.
        self.assertEqual(spans('greedy-rules-runs', 'filter_then_none'),
                         [(0, 1), (0, 2), (1, 2), (2, 3), (3, 4), (3, 5), (4, 5), (3, 6), (4, 6), (5, 6)])
        self.assertEqual(spans('greedy-rules-runs', 'none_then_filter'), [(0, 2), (2, 3), (3, 6)])
        # The later registration's patterns compete in the same pass: `a b a` gives (1, 4) and
        # `a a b` gives (0, 3), each displacing spans of the earlier `a+`.
        self.assertEqual(spans('greedy-rules-runs', 'first_then_longest'), [(1, 4), (4, 6), (0, 1)])
        self.assertEqual(spans('greedy-rules-runs', 'longest_then_first'), [(0, 3), (3, 6)])
        self.assertEqual(spans('greedy-rules-runs', 'three_registrations'), [(0, 2), (2, 3), (3, 5), (5, 6)])
        # A registration without patterns sets or clears the filter.
        self.assertEqual(spans('greedy-rules-runs', 'empty_sets_filter'), [(3, 6), (0, 2)])
        self.assertEqual(len(spans('greedy-rules-runs', 'empty_clears_filter')), 9)
        # LONGEST prefers the earlier of two equally long spans even when spaCy finds the
        # later one first.
        found = [(start, end) for name, start, end in cases['greedy-rules-discovery-order'].unfiltered
                 if name == 'found_late_start_first_longest']
        self.assertLess(found.index((1, 3)), found.index((0, 2)))
        self.assertEqual(spans('greedy-rules-discovery-order', 'found_late_start_first_longest'), [(0, 2), (3, 5)])
        # Matches of unfiltered rules come before those of every filtered rule.
        order = [name for name, _, _ in cases['greedy-rules-runs'].expected]
        unfiltered_rules = {'plain_a', 'plain_b', 'filter_then_none', 'empty_clears_filter'}
        last_unfiltered = max(index for index, name in enumerate(order) if name in unfiltered_rules)
        self.assertTrue(all(name in unfiltered_rules for name in order[:last_unfiltered]))
        # A model-annotated document: runs of proper nouns.
        self.assertEqual(spans('greedy-rules-pipeline', 'proper_nouns_first'), [(0, 2), (3, 4), (5, 8)])
        self.assertEqual(spans('greedy-rules-pipeline', 'proper_nouns_longest'), [(5, 8), (0, 2), (3, 4)])


class GreedyRegistrationReferenceTests(unittest.TestCase):
    """Pinned spaCy behaviour that a list of successful registrations cannot record: rejected
    `greedy` values, rejected registrations and removal."""

    def setUp(self) -> None:
        self.vocab = spacy.blank('en').vocab
        self.doc = make_doc(self.vocab, words=['a', 'a', 'b', 'a', 'a', 'a'])
        self.a_run = [official(Pattern([word('a', Repetition('one_or_more'))]))]
        self.single_b = [official(Pattern([word('b')]))]

    def spans(self, matcher: Matcher) -> list[Span]:
        return [(start, end) for _, start, end in matcher(self.doc)]

    def test_spacy_rejects_greedy_values_other_than_first_and_longest(self) -> None:
        unfiltered = Matcher(self.vocab, validate=True)
        unfiltered.add('r', self.a_run)
        for value in ('GREEDY', 'first', 'longest', 'First', ''):
            matcher = Matcher(self.vocab, validate=True)
            with self.assertRaisesRegex(ValueError, 'E947'):
                matcher.add('r', self.single_b, greedy=value)
            # The rejected call stored neither its pattern nor a filter.
            matcher.add('r', self.a_run)
            self.assertEqual(self.spans(matcher), self.spans(unfiltered), value)

    def test_spacy_checks_the_greedy_value_before_the_patterns(self) -> None:
        matcher = Matcher(self.vocab, validate=True)
        with self.assertRaisesRegex(ValueError, 'E947'):
            matcher.add('r', [[]], greedy='GREEDY')

    def test_spacy_keeps_the_filter_when_a_later_registration_is_rejected(self) -> None:
        matcher = Matcher(self.vocab, validate=True)
        matcher.add('r', self.a_run, greedy=Greedy.LONGEST)
        with self.assertRaisesRegex(ValueError, 'E012'):
            matcher.add('r', [[]])
        with self.assertRaisesRegex(ValueError, 'E012'):
            matcher.add('r', [[]], greedy=Greedy.FIRST)
        with self.assertRaisesRegex(ValueError, 'E947'):
            matcher.add('r', self.single_b, greedy='first')
        self.assertEqual(self.spans(matcher), [(3, 6), (0, 2)])

    def test_spacy_takes_the_filter_from_the_registration_after_a_removal(self) -> None:
        matcher = Matcher(self.vocab, validate=True)
        matcher.add('r', self.a_run, greedy=Greedy.LONGEST)
        matcher.remove('r')
        matcher.add('r', self.a_run)
        self.assertEqual(len(self.spans(matcher)), 9)
        matcher.remove('r')
        matcher.add('r', self.a_run, greedy=Greedy.FIRST)
        self.assertEqual(self.spans(matcher), [(0, 2), (3, 6)])

    def test_spacy_keeps_the_filters_of_other_rules_after_a_removal(self) -> None:
        matcher = Matcher(self.vocab, validate=True)
        matcher.add('longest', self.a_run, greedy=Greedy.LONGEST)
        matcher.add('first', self.a_run, greedy=Greedy.FIRST)
        matcher.remove('longest')
        self.assertEqual(self.spans(matcher), [(0, 2), (3, 6)])

    def test_spacy_registers_a_new_rule_without_patterns(self) -> None:
        matcher = Matcher(self.vocab, validate=True)
        matcher.add('new', [], greedy=Greedy.FIRST)
        self.assertIn('new', matcher)
        self.assertEqual(len(matcher), 1)
        self.assertEqual(matcher(self.doc), [])


if __name__ == '__main__':
    unittest.main()
