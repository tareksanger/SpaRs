"""Freeze Token Matcher expectations from the pinned official spaCy runtime."""
import argparse
from dataclasses import asdict, dataclass
import hashlib
import json
from itertools import product
from pathlib import Path
from typing import Literal

import spacy
import thinc
from spacy.matcher import Matcher
from spacy.tokens import Doc as make_doc

from dependency_match_reference import ATTRIBUTES, Constraint, Equals, Membership, Versions, StringOperator, attribute_name
from reference_types import Doc, Language, SpanRecord, TokenRecord, span_records, token_records


@dataclass(frozen=True)
class Repetition:
    kind: Literal['once', 'optional', 'zero_or_more', 'one_or_more', 'negated'] = 'once'


@dataclass(frozen=True)
class Range:
    min: int
    max: int | None
    kind: Literal['range'] = 'range'


@dataclass(frozen=True)
class Item:
    constraints: list[Constraint]
    repetition: Repetition | Range = Repetition()


@dataclass(frozen=True)
class Pattern:
    tokens: list[Item]


@dataclass(frozen=True)
class Rule:
    name: str
    patterns: list[Pattern]


@dataclass(frozen=True)
class Match:
    rule: str
    start: int
    end: int


@dataclass(frozen=True)
class Case:
    id: str
    text: str
    tokens: list[TokenRecord]
    sentences: list[SpanRecord]
    entities: list[SpanRecord]
    noun_chunks: list[SpanRecord]
    rules: list[Rule]
    expected: list[Match]


@dataclass(frozen=True)
class Fixture:
    versions: Versions
    model: str
    source_sha256: str
    cases: list[Case]


def official(pattern: Pattern) -> list[dict[str, str | StringOperator]]:
    result: list[dict[str, str | StringOperator]] = []
    for item in pattern.tokens:
        attrs: dict[str, str | StringOperator] = {}
        for constraint in item.constraints:
            attribute = ATTRIBUTES[constraint.attribute]
            if attribute in attrs:
                raise ValueError('Official converter requires distinct attributes per item')
            predicate = constraint.predicate
            if isinstance(predicate, Equals):
                value: str | StringOperator = predicate.value
            elif predicate.kind == 'in':
                value = {'IN': predicate.values}
            elif predicate.kind == 'not_in':
                value = {'NOT_IN': predicate.values}
            elif predicate.kind == 'morph_superset':
                value = {'IS_SUPERSET': predicate.values}
            else:
                value = {'INTERSECTS': predicate.values}
            attrs[attribute] = value
        repeat = item.repetition
        if isinstance(repeat, Range):
            if repeat.min < 0 or (repeat.max is not None and repeat.max < repeat.min):
                raise ValueError('Invalid repetition range')
            attrs['OP'] = '{' + str(repeat.min) + ',' + ('' if repeat.max is None else str(repeat.max)) + '}'
        elif repeat.kind != 'once':
            attrs['OP'] = {'once': '1', 'optional': '?', 'zero_or_more': '*', 'one_or_more': '+', 'negated': '!'}[repeat.kind]
        result.append(attrs)
    return result


def word(text: str, repeat: Repetition | Range = Repetition()) -> Item:
    return Item([Constraint('text', Equals(text))], repeat)


def rules() -> list[Rule]:
    result = [Rule(kind, [Pattern([word('a', Repetition(kind))])]) for kind in ('once', 'optional', 'zero_or_more', 'one_or_more', 'negated')]
    for low, high in [(0, 0), (0, 2), (1, 1), (1, 3), (2, 3), (2, None)]:
        result.append(Rule(f'range_{low}_{high}', [Pattern([word('a', Range(low, high))])]))
    for name, items in [
        ('optional_pair', [word('a', Repetition('optional')), word('a', Repetition('optional'))]),
        ('star_pair', [word('a', Repetition('zero_or_more')), word('a', Repetition('zero_or_more'))]),
        ('plus_then_b', [word('a', Repetition('one_or_more')), word('b')]),
        ('a_then_optional_b', [word('a'), word('b', Repetition('optional'))]),
        ('optional_a_then_b', [word('a', Repetition('optional')), word('b')]),
        ('star_a_then_b', [word('a', Repetition('zero_or_more')), word('b')]),
        ('a_negated_b', [word('a'), word('b', Repetition('negated'))]),
        ('wildcard', [Item([])]),
        ('never_negated_wildcard', [Item([], Repetition('negated'))]),
        ('conjunction', [Item([Constraint('text', Equals('a')), Constraint('lemma', Equals('a'))])]),
    ]:
        result.append(Rule(name, [Pattern(items)]))
    duplicate = Pattern([word('a')])
    result.extend([Rule('duplicates', [duplicate, duplicate]), Rule('later', [Pattern([word('b')])]), Rule('duplicates', [Pattern([word('a'), word('b')]), duplicate])])
    for attr, value in [('text', 'a'), ('norm', 'a'), ('lemma', 'a'), ('pos', 'NOUN'), ('tag', 'NN'), ('dep', 'ROOT'), ('morphology', 'Number=Sing')]:
        for predicate in [Equals(value), Membership('in', [value]), Membership('not_in', [value])]:
            result.append(Rule(f'{attr}_{predicate.kind}', [Pattern([Item([Constraint(attribute_name(attr), predicate)])])]))
    result.extend([
        Rule('morph_superset', [Pattern([Item([Constraint('morphology', Membership('morph_superset', ['Case=Nom']))])])]),
        Rule('morph_intersects', [Pattern([Item([Constraint('morphology', Membership('morph_intersects', ['Case=Nom', 'Number=Sing']))])])]),
        Rule('empty_membership', [Pattern([Item([Constraint('text', Membership('in', []))])])]),
    ])
    return result


def case(nlp: Language, case_id: str, text: str, doc: Doc, patterns: list[Rule] | None = None) -> Case:
    if patterns is None:
        patterns = rules()
    matcher = Matcher(nlp.vocab, validate=True)
    for rule in patterns:
        matcher.add(rule.name, [official(pattern) for pattern in rule.patterns])
    expected = [Match(nlp.vocab.strings[key], start, end) for key, start, end in matcher(doc)]
    return Case(case_id, text, token_records(doc, text), span_records(doc.sents), [], [], patterns, expected)


def exhaustive_rules() -> list[Rule]:
    repetitions = [Repetition(kind) for kind in ('once', 'optional', 'zero_or_more', 'one_or_more', 'negated')]
    return [Rule('_'.join(repeat.kind for repeat in sequence), [Pattern([word('a', repeat) for repeat in sequence])])
            for length in range(1, 4) for sequence in product(repetitions, repeat=length)]


def branching_rules() -> list[Rule]:
    result: list[Rule] = []
    for length in (9, 16):
        for alternating in (False, True):
            items = [word('a', Repetition('zero_or_more' if alternating and index % 2 else 'optional')) for index in range(length)]
            name = f'branching_{length}_{alternating}'
            result.append(Rule(name, [Pattern(items)]))
            result.append(Rule(name + '_failing_tail', [Pattern([*items, word('missing')])]))
    return result


def generate_exhaustive(nlp: Language, branching: bool = False) -> Fixture:
    patterns = branching_rules() if branching else exhaustive_rules()
    cases: list[Case] = []
    for length in range(5):
        for sequence in product(('a', 'b'), repeat=length):
            words: list[str] = list(sequence)
            doc = make_doc(nlp.vocab, words=words, spaces=[index + 1 < length for index in range(length)],
                           heads=[0] * length, deps=['ROOT' if index == 0 else 'dep' for index in range(length)],
                           pos=['NOUN'] * length, tags=['NN'] * length, lemmas=words, morphs=[''] * length)
            cases.append(case(nlp, 'binary-' + (''.join(words) or 'empty'), ' '.join(words), doc, patterns))
    source = Path(spacy.__file__).parent / 'matcher' / 'matcher.pyx'
    return Fixture(Versions(spacy.__version__, thinc.__version__), 'en_core_web_md 3.8.0', hashlib.sha256(source.read_bytes()).hexdigest(), cases)


def generate(exhaustive: bool = False, branching: bool = False) -> Fixture:
    if (spacy.__version__, thinc.__version__) != ('3.8.14', '8.3.13'):
        raise ValueError('Use the pinned reference environment')
    nlp = spacy.load('en_core_web_md')
    if nlp.meta['version'] != '3.8.0':
        raise ValueError('Use en_core_web_md 3.8.0')
    if exhaustive or branching:
        return generate_exhaustive(nlp, branching)
    cases: list[Case] = []
    for case_id, text in [('ordinary', 'Alice saw Bob and Carol.'), ('sentences', 'Alice left. Bob stayed.'), ('unicode', 'Zoë sees 👩🏽‍💻 today.'), ('empty', '')]:
        cases.append(case(nlp, case_id, text, nlp(text)))
    for index, words in enumerate([['a'], ['b'], ['a', 'a', 'a'], ['a', 'a', 'b', 'a', 'b'], ['b', 'a', 'b', 'b', 'a'], ['a', 'b', 'a', 'a']]):
        count = len(words)
        doc = make_doc(nlp.vocab, words=words, spaces=[True] * (count - 1) + [False], heads=[0] * count, deps=['ROOT'] + ['dep'] * (count - 1), pos=['NOUN'] * count, tags=['NN'] * count, lemmas=words, morphs=['Case=Acc,Nom|Number=Sing'] * count)
        cases.append(case(nlp, f'repetition-{index}', ' '.join(words), doc))
    source = Path(spacy.__file__).parent / 'matcher' / 'matcher.pyx'
    return Fixture(Versions(spacy.__version__, thinc.__version__), 'en_core_web_md 3.8.0', hashlib.sha256(source.read_bytes()).hexdigest(), cases)


def write_fixture(output: Path, fixture: Fixture) -> None:
    output.parent.mkdir(parents=True, exist_ok=True)
    with output.open('x', encoding='utf-8') as stream:
        stream.write(json.dumps(asdict(fixture), ensure_ascii=False) + '\n')


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    modes = parser.add_mutually_exclusive_group()
    modes.add_argument('--exhaustive', action='store_true')
    modes.add_argument('--branching', action='store_true')
    args = parser.parse_args()
    output: object = args.output
    if not isinstance(output, Path):
        raise TypeError('Output must be a path')
    if output.exists():
        parser.error('Output exists; frozen expectations stay unchanged')
    exhaustive: object = args.exhaustive
    if not isinstance(exhaustive, bool):
        raise TypeError('Exhaustive flag must be boolean')
    branching: object = args.branching
    if not isinstance(branching, bool):
        raise TypeError('Branching flag must be boolean')
    write_fixture(output, generate(exhaustive, branching))


if __name__ == '__main__':
    main()
