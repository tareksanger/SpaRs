"""Freeze exact ordered PhraseMatcher behavior from verified official sources."""
import argparse
from dataclasses import asdict, dataclass
from collections.abc import Sequence
import hashlib
import importlib.metadata
import json
from pathlib import Path
import random

import spacy
from spacy.matcher import PhraseMatcher
from spacy.tokens import Doc as make_doc
from reference_types import Vocab
from json_types import read_json, json_object, json_string

ROOT = Path(__file__).resolve().parent.parent

@dataclass(frozen=True)
class Operation:
    action: str
    rule: str
    patterns: list[list[str]]

@dataclass(frozen=True)
class Match:
    rule: str
    start: int
    end: int

@dataclass(frozen=True)
class State:
    error: str | None
    rules: list[str]
    matches: list[Match]

@dataclass(frozen=True)
class Case:
    id: str
    words: list[str]
    spaces: list[bool]
    operations: list[Operation]
    states: list[State]

@dataclass(frozen=True)
class Probe:
    attribute: str
    error: str | None
    registered: bool
    matches: list[Match]

@dataclass(frozen=True)
class Fixture:
    format_version: int
    versions: dict[str, str]
    sources: dict[str, str]
    cases: list[Case]
    probes: list[Probe]


def verify_sources() -> tuple[dict[str, str], dict[str, str]]:
    lock = json_object(read_json(ROOT / 'reference/source-lock.json'))
    extra = json_object(read_json(ROOT / 'reference/phrase-source-lock.json'))
    versions: dict[str, str] = {}
    sources: dict[str, str] = {}
    for name, selected in [('spacy', ['spacy/matcher/phrasematcher.pyx', 'spacy/tests/matcher/test_phrase_matcher.py', 'spacy/strings.pyx', 'spacy/symbols.pyx']), ('preshed', ['preshed/maps.pyx'])]:
        record = json_object((lock if name == 'spacy' else extra)[name])
        distribution = importlib.metadata.distribution(name)
        version = json_string(record['version'])
        if distribution.version != version:
            raise ValueError(f'Unexpected {name} version')
        versions[name] = version
        files = json_object(record['files'])
        for source in (list(files) if name == "preshed" else selected):
            digest = hashlib.sha256(Path(str(distribution.locate_file(source))).read_bytes()).hexdigest()
            if digest != json_string(files[source]):
                raise ValueError(f'Source mismatch: {source}')
            if source in selected:
                sources[source] = digest
    if versions['spacy'] != '3.8.14':
        raise ValueError('Expected spaCy 3.8.14')
    return versions, sources


def checked_integer(value: object) -> int:
    if isinstance(value, bool) or not isinstance(value, int):
        raise ValueError('Expected integer match IDs and indices')
    return value


def checked_label(value: object) -> str:
    if not isinstance(value, str):
        raise ValueError('Expected string rule identity')
    return value


def convert_matches(raw: Sequence[tuple[object, object, object]], vocab: Vocab, length: int) -> list[Match]:
    matches: list[Match] = []
    for raw_key, raw_start, raw_end in raw:
        key, start, end = (checked_integer(v) for v in (raw_key, raw_start, raw_end))
        if not 0 <= start < end <= length:
            raise ValueError('Invalid match bounds')
        label = checked_label(vocab.strings[key])
        matches.append(Match(label, start, end))
    return matches


def build(holdout: bool) -> Fixture:
    versions, sources = verify_sources()
    nlp = spacy.blank('en')
    cases: list[Case] = []
    def capture(name: str, words: list[str], operations: list[Operation], spaces: list[bool] | None = None) -> None:
        gaps = spaces if spaces is not None else [True] * len(words)
        doc = make_doc(nlp.vocab, words=words, spaces=gaps)
        matcher = PhraseMatcher(nlp.vocab)
        labels: list[str] = []
        states: list[State] = []
        for op in operations:
            if op.rule not in labels:
                labels.append(op.rule)
            error: str | None = None
            try:
                if op.action == 'add':
                    matcher.add(op.rule, [make_doc(nlp.vocab, words=p) for p in op.patterns])
                elif op.action == 'remove':
                    matcher.remove(op.rule)
                else:
                    raise ValueError('Unknown operation')
            except (ValueError, KeyError) as exc:
                error = type(exc).__name__
            registered = [label for label in labels if label in matcher]
            if len(registered) != len(matcher):
                raise ValueError('Rule count mismatch')
            states.append(State(error, registered, convert_matches(matcher(doc), nlp.vocab, len(doc))))
        cases.append(Case(name, words, gaps, operations, states))
    def add(label: str, *patterns: list[str]) -> Operation:
        return Operation('add', label, list(patterns))
    def remove(label: str) -> Operation:
        return Operation('remove', label, [])
    if not holdout:
        capture('nested-overlaps', ['a', 'a', 'a', 'b'], [add('short', ['a']), add('long', ['a', 'a']), add('tail', ['a', 'b']), add('short', ['a'], ['a', 'a']), remove('long'), remove('short'), add('short', ['a'])])
        capture('empty', [], [add('none'), add('empty', []), remove('none'), remove('missing')])
        capture('empty-patterns', ['a'], [add('empty', []), add('a', [], ['a'], []), remove('a'), remove('empty')])
        capture('unicode', ['É', 'é', 'é', '🙂', '東京', 'ß', 'SS'], [add('accent', ['é'], ['é']), add('emoji', ['🙂', '東京']), add('case', ['ss'], ['ß'])])
        capture('whitespace', ['a', '\n', 'b', '.', 'a', 'b'], [add('space', ['a', '\n', 'b']), add('across', ['b', '.', 'a']), add('ab', ['a', 'b'])], [False, False, False, True, False, False])
        capture('shared-prefix-removal', ['a','b','c','a','b'], [add('A', ['a'], ['a','b'], ['a','b','c']), add('B', ['a','b']), remove('A'), add('A', ['a','b','c']), remove('B'), remove('A'), add('B', ['a','b'])])
        labels = ['', 'IS_ALPHA', 'ORTH', 'A', 'B', 'C', *[f'label-{i}' for i in range(36)]]
        capture('terminal-order', ['x','x'], [*[add(s, ['x']) for s in labels], *[remove(s) for s in labels[::3]], *[add(s, ['x'], ['x']) for s in reversed(labels)], *[remove(s) for s in labels], add('reset', ['x'])])
    rng = random.Random(90210 if holdout else 414)
    for index in range(24 if holdout else 12):
        words = [rng.choice(['a','b','c','é','🙂']) for _ in range(12 + index)]
        ops: list[Operation] = []
        for j in range(30):
            label = rng.choice(['', 'IS_ALPHA', 'A', 'B', 'C', 'D', 'E', 'F'])
            start = rng.randrange(len(words))
            ops.append(remove(label) if j % 5 == 4 else add(label, words[start:start+rng.randrange(1,5)]))
        capture(f"{'holdout' if holdout else 'development'}-{index}", words, ops)
    probes: list[Probe] = []
    if not holdout:
        for attr in ['ORTH', 'TEXT', 'orth', 'TAG', 'POS', 'LEMMA', 'DEP', 'MORPH', 'INVALID']:
            matcher = None
            error = None
            matches: list[Match] = []
            try:
                matcher = PhraseMatcher(nlp.vocab, attr=attr)
                matcher.add('probe', [make_doc(nlp.vocab, words=['a'])])
                matches = convert_matches(matcher(make_doc(nlp.vocab, words=['a'])), nlp.vocab, 1)
            except ValueError as exc:
                error = type(exc).__name__
            probes.append(Probe(attr, error, matcher is not None and 'probe' in matcher, matches))
    return Fixture(1, versions, sources, cases, probes)


@dataclass(frozen=True)
class EdgeFixture:
    format_version: int
    versions: dict[str, str]
    sources: dict[str, str]
    sentence_starts: list[bool]
    sentence: Case
    probes: list[Probe]


def build_edges() -> EdgeFixture:
    versions, sources = verify_sources()
    nlp = spacy.blank('en')
    words = ['a', '.', 'b']
    spaces = [False, True, False]
    starts = [True, False, True]
    doc = make_doc(nlp.vocab, words=words, spaces=spaces, sent_starts=starts)
    matcher = PhraseMatcher(nlp.vocab)
    matcher.add('cross', [make_doc(nlp.vocab, words=words)])
    state = State(None, ['cross'], convert_matches(matcher(doc), nlp.vocab, len(doc)))
    case = Case('explicit-sentence-boundary', words, spaces, [Operation('add', 'cross', [words])], [state])
    probes: list[Probe] = []
    tagged = make_doc(nlp.vocab, words=['a'], tags=['NN'])
    untagged = make_doc(nlp.vocab, words=['a'])
    matcher = PhraseMatcher(nlp.vocab, attr='TAG')
    try:
        matcher.add('partial', [tagged, untagged])
    except ValueError:
        probes.append(Probe('TAG-partial', 'ValueError', 'partial' in matcher, convert_matches(matcher(tagged), nlp.vocab, 1)))
    else:
        raise ValueError('Expected missing-annotation failure')
    matcher = PhraseMatcher(nlp.vocab)
    try:
        matcher.add('invalid', untagged)
    except ValueError:
        probes.append(Probe('ORTH-single-doc', 'ValueError', 'invalid' in matcher, convert_matches(matcher(untagged), nlp.vocab, 1)))
    else:
        raise ValueError('Expected document-collection failure')
    return EdgeFixture(1, versions, sources, starts, case, probes)


class Options(argparse.Namespace):
    output: Path = Path('target/reports/phrase-reference.json')
    holdout: bool = False
    edges: bool = False


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--holdout', action='store_true')
    parser.add_argument('--edges', action='store_true')
    args = Options()
    parser.parse_args(namespace=args)
    output = args.output
    if output.exists():
        raise ValueError('Refusing to overwrite frozen output')
    fixture = build_edges() if args.edges else build(bool(args.holdout))
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(asdict(fixture), ensure_ascii=False, indent=2) + '\n')
    if isinstance(fixture, Fixture):
        print(f'{len(fixture.cases)} cases; {sum(len(c.states) for c in fixture.cases)} states; {sum(len(s.matches) for c in fixture.cases for s in c.states)} matches')
    else:
        print('1 explicit sentence-boundary case; 2 error-state probes')

if __name__ == '__main__':
    main()
