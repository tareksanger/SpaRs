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
from spacy.attrs import DEP, LEMMA, MORPH, NORM, POS, TAG
from spacy.matcher import PhraseMatcher
from spacy.tokens import Doc as make_doc
from reference_types import Doc, Language, Vocab
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


@dataclass(frozen=True)
class AnnotatedToken:
    """One token; None means spaCy stores no value (key 0), and morph "" is the empty analysis."""
    word: str
    space: bool
    norm: str
    lemma: str | None
    pos: str | None
    tag: str | None
    dep: str | None
    morph: str | None

@dataclass(frozen=True)
class AnnotatedOperation:
    action: str
    rule: str
    patterns: list[list[AnnotatedToken]]

@dataclass(frozen=True)
class AnnotatedState:
    error: str | None
    rules: list[str]
    # spaCy's stored pattern keys for each rule in `rules`, as strings, sorted.
    patterns: list[list[list[str]]]
    matches: list[Match]

@dataclass(frozen=True)
class AnnotatedCase:
    id: str
    attribute: str
    tokens: list[AnnotatedToken]
    operations: list[AnnotatedOperation]
    states: list[AnnotatedState]

@dataclass(frozen=True)
class AnnotationFixture:
    format_version: int
    versions: dict[str, str]
    sources: dict[str, str]
    cases: list[AnnotatedCase]


ANNOTATION_ATTRIBUTES = ['NORM', 'LEMMA', 'POS', 'TAG', 'DEP', 'MORPH']
ANNOTATION_IDS = {'NORM': NORM, 'LEMMA': LEMMA, 'POS': POS, 'TAG': TAG, 'DEP': DEP, 'MORPH': MORPH}
# (word, norm, lemma, pos, tag, dep, morph) for project-authored synthetic tokens.
type TokenSpec = tuple[str, str, str | None, str | None, str | None, str | None, str | None]


def record_tokens(doc: Doc) -> list[AnnotatedToken]:
    """Record values with spaCy's own missing-value test: a zero key is no value."""
    keys = doc.to_array([LEMMA, POS, TAG, DEP, MORPH])
    tokens: list[AnnotatedToken] = []
    for token, row in zip(doc, keys, strict=True):
        lemma, pos, tag, dep, morph = (int(value) for value in row)
        tokens.append(AnnotatedToken(
            token.text, token.whitespace_ == ' ', token.norm_,
            token.lemma_ if lemma else None, token.pos_ if pos else None,
            token.tag_ if tag else None, token.dep_ if dep else None,
            str(token.morph) if morph else None))
    return tokens


def annotated_doc(vocab: Vocab, tokens: list[AnnotatedToken]) -> Doc:
    doc = make_doc(vocab, words=[t.word for t in tokens], spaces=[t.space for t in tokens])
    for token, spec in zip(doc, tokens, strict=True):
        token.norm_ = spec.norm
        if spec.lemma is not None:
            token.lemma_ = spec.lemma
        if spec.pos is not None:
            token.pos_ = spec.pos
        if spec.tag is not None:
            token.tag_ = spec.tag
        if spec.dep is not None:
            token.dep_ = spec.dep
        if spec.morph is not None:
            token.set_morph(spec.morph)
    if record_tokens(doc) != tokens:
        raise ValueError('spaCy did not store the requested annotations unchanged')
    return doc


def synthetic(specs: list[TokenSpec]) -> list[AnnotatedToken]:
    return [AnnotatedToken(word, True, norm, lemma, pos, tag, dep, morph) for word, norm, lemma, pos, tag, dep, morph in specs]


def stored_patterns(matcher: PhraseMatcher, label: str) -> list[tuple[int, ...]]:
    """Read spaCy's private stored keys from its pickling data, checking each key."""
    stored = matcher.__reduce__()[1][1]
    return [tuple(checked_integer(key) for key in keyword) for keyword in stored[label]]


def capture_annotated(nlp: Language, name: str, attribute: str, tokens: list[AnnotatedToken], operations: list[AnnotatedOperation]) -> AnnotatedCase:
    vocab = nlp.vocab
    doc = annotated_doc(vocab, tokens)
    matcher = PhraseMatcher(vocab, attr=attribute)
    labels: list[str] = []
    states: list[AnnotatedState] = []
    for op in operations:
        if op.rule not in labels:
            labels.append(op.rule)
        error: str | None = None
        try:
            if op.action == 'add':
                matcher.add(op.rule, [annotated_doc(vocab, pattern) for pattern in op.patterns])
            elif op.action == 'remove':
                matcher.remove(op.rule)
            else:
                raise ValueError('Unknown operation')
        except (ValueError, KeyError) as exc:
            error = type(exc).__name__
        registered = [label for label in labels if label in matcher]
        if len(registered) != len(matcher):
            raise ValueError('Rule count mismatch')
        patterns = [sorted([checked_label(vocab.strings[checked_integer(key)]) for key in keyword] for keyword in stored_patterns(matcher, label)) for label in registered]
        states.append(AnnotatedState(error, registered, patterns, convert_matches(matcher(doc), vocab, len(doc))))
    return AnnotatedCase(name, attribute, tokens, operations, states)


def build_annotations() -> AnnotationFixture:
    versions, sources = verify_sources()
    nlp = spacy.load('en_core_web_md')
    if nlp.meta.get('version') != '3.8.0':
        raise ValueError('Expected en_core_web_md 3.8.0')
    versions['en_core_web_md'] = '3.8.0'
    cases: list[AnnotatedCase] = []
    def add(label: str, *patterns: list[AnnotatedToken]) -> AnnotatedOperation:
        return AnnotatedOperation('add', label, list(patterns))
    def remove(label: str) -> AnnotatedOperation:
        return AnnotatedOperation('remove', label, [])
    # Model-annotated input and patterns: each pattern is processed on its own.
    text = 'The dogs ran home, and the dog runs fast. We were running and they ran. Gon na go?'
    parsed = record_tokens(nlp(text))
    phrase = {value: record_tokens(nlp(value)) for value in ['the dog', 'the dogs', 'A cat runs', 'were running', 'they run', 'is running', 'going to', 'gonna', 'We ran', 'home ,', 'fast .']}
    plain = record_tokens(nlp.make_doc('the dog'))
    for attribute in ANNOTATION_ATTRIBUTES:
        cases.append(capture_annotated(nlp, f'pipeline-{attribute.lower()}', attribute, parsed, [
            add('np', phrase['the dog'], phrase['A cat runs']),
            add('verb', phrase['were running'], phrase['they run']),
            add('np', phrase['the dog'], phrase['the dogs']),
            add('slang', phrase['going to'], phrase['gonna']),
            add('np', plain),
            # spaCy keeps patterns before the rejected one; SpaRs keeps none.
            add('mixed', phrase['the dogs'], plain),
            add('mixed', phrase['the dogs']),
            remove('np'),
            add('np', phrase['We ran'], phrase['home ,'], phrase['fast .']),
            remove('verb'),
            add('fresh', plain),
            add('fresh', phrase['is running']),
        ]))
    # Synthetic tokens with missing values, the empty morphological analysis and
    # values that differ from the token text.
    sentence = synthetic([
        ('Cats', 'cats', 'cat', 'NOUN', 'NNS', 'nsubj', 'Number=Plur'),
        ('sleep', 'sleep', 'sleep', 'VERB', 'VBP', 'ROOT', 'Tense=Pres|VerbForm=Fin'),
        ('here', 'here', None, None, None, None, None),
        ('now', 'now', 'now', 'ADV', 'RB', 'advmod', ''),
        ('cats', 'cats', 'cat', 'NOUN', 'NNS', 'dobj', 'Number=Plur'),
        ('!', '!', None, 'PUNCT', None, 'punct', ''),
        ('Gon', 'going', 'go', 'VERB', 'VBG', 'aux', 'Tense=Pres|VerbForm=Part'),
        ('na', 'to', 'to', 'PART', 'TO', 'aux', ''),
    ])
    full = synthetic([('Cat', 'cats', 'cat', 'NOUN', 'NNS', 'nsubj', 'Number=Plur'), ('sleeps', 'sleep', 'sleep', 'VERB', 'VBP', 'ROOT', 'Tense=Pres|VerbForm=Fin')])
    partial = synthetic([('there', 'here', None, None, None, None, None), ('then', 'now', 'now', 'ADV', 'RB', 'advmod', '')])
    empty_morph = synthetic([('?', '!', 'x', 'PUNCT', '.', 'punct', '')])
    missing_tail = synthetic([('dogs', 'cats', 'cat', 'NOUN', 'NNS', 'dobj', 'Number=Plur'), ('.', '!', None, None, None, None, None)])
    slang = synthetic([('going', 'going', 'go', 'VERB', 'VBG', 'aux', 'Tense=Pres|VerbForm=Part'), ('to', 'to', 'to', 'PART', 'TO', 'aux', '')])
    unannotated = synthetic([('Cats', 'cats', None, None, None, None, None), ('sleep', 'sleep', None, None, None, None, None)])
    bare = [AnnotatedToken(t.word, t.space, t.norm, None, None, None, None, None) for t in sentence]
    operations = [
        add('full', full),
        add('partial', partial, empty_morph),
        add('tail', missing_tail, slang),
        add('missing', unannotated),
        add('missing', partial),
        add('mixed', full, unannotated),
        add('mixed', full),
        add('empty', []),
        add('full', unannotated, full),
        remove('partial'),
        add('partial', partial),
    ]
    for attribute in ANNOTATION_ATTRIBUTES:
        cases.append(capture_annotated(nlp, f'missing-values-{attribute.lower()}', attribute, sentence, operations))
        cases.append(capture_annotated(nlp, f'unannotated-input-{attribute.lower()}', attribute, bare, operations))
    return AnnotationFixture(1, versions, sources, cases)


@dataclass(frozen=True)
class LexicalState:
    error: str | None
    rules: list[str]
    # spaCy's stored integer keys for each rule in `rules`: 0 or 1 for a flag, or a length; sorted.
    patterns: list[list[list[int]]]
    matches: list[Match]

@dataclass(frozen=True)
class LexicalCase:
    id: str
    attribute: str
    words: list[str]
    spaces: list[bool]
    operations: list[Operation]
    states: list[LexicalState]

@dataclass(frozen=True)
class LexicalFixture:
    format_version: int
    versions: dict[str, str]
    sources: dict[str, str]
    cases: list[LexicalCase]


LEXICAL_ATTRIBUTES = ['IS_ALPHA', 'IS_ASCII', 'IS_DIGIT', 'IS_LOWER', 'IS_UPPER', 'IS_TITLE', 'IS_PUNCT', 'IS_SPACE',
                      'IS_BRACKET', 'IS_QUOTE', 'IS_LEFT_PUNCT', 'IS_RIGHT_PUNCT', 'IS_CURRENCY', 'IS_STOP',
                      'LIKE_NUM', 'LIKE_URL', 'LIKE_EMAIL', 'LENGTH']


def build_lexical() -> LexicalFixture:
    from dependency_match_reference import LENGTH_PIPELINE_TEXT, MORE_FLAG_PIPELINE_TEXT
    versions, sources = verify_sources()
    nlp = spacy.load('en_core_web_md')
    if nlp.meta.get('version') != '3.8.0':
        raise ValueError('Expected en_core_web_md 3.8.0')
    versions['en_core_web_md'] = '3.8.0'
    vocab = nlp.vocab
    # Lexical values come from the model vocabulary, so tokenized documents suffice.
    # Each flag is true for some of these words and false for others; lengths vary too.
    words = ['Hello', 'hello', 'HELLO', '123', '\u0661\u0662\u0663', '\u00b2', '...', '\u2014', '$', '\u20ac', '(', ')',
             '"', '\u00ab', '\u00bb', 'the', 'ten', '3rd', 'https://spacy.io', 'example.org', 'a@b.com', '\u00a0', '\n',
             '\U0001f642', 'e\u0301', 'supercalifragilistic', 'caf\u00e9', '1,000', 'Twelfth', '.']
    # The double space becomes a whitespace token, so IS_SPACE varies in the text too.
    parsed = nlp.make_doc('  '.join([MORE_FLAG_PIPELINE_TEXT, LENGTH_PIPELINE_TEXT]))
    inputs = [('words', words, [True] * (len(words) - 1) + [False]),
              ('text', [t.text for t in parsed], [t.whitespace_ == ' ' for t in parsed])]
    def add(label: str, *patterns: list[str]) -> Operation:
        return Operation('add', label, list(patterns))
    # No removals: preshed reserves the keys 0 and 1, which are flag values and a
    # one-character length. spaCy 3.8.14 removal then frees a trie node that stays
    # reachable, so later matching reads freed memory or loses rules sharing the prefix.
    operations = [
        add('one', ['Hello'], ['123']),
        add('two', ['ten', 'the'], ['https://spacy.io', 'a@b.com']),
        add('three', ['$', '(', '"'], ['hello', 'Hello', 'HELLO'], []),
        add('one', ['Hello'], ['\u00a0']),
        add('long', ['supercalifragilistic', 'caf\u00e9', '1,000', '.'], ['\u0661\u0662\u0663', '\u2014', 'Twelfth']),
    ]
    cases: list[LexicalCase] = []
    for attribute in LEXICAL_ATTRIBUTES:
        for name, input_words, spaces in inputs:
            doc = make_doc(vocab, words=input_words, spaces=spaces)
            matcher = PhraseMatcher(vocab, attr=attribute)
            labels: list[str] = []
            states: list[LexicalState] = []
            for op in operations:
                if op.rule not in labels:
                    labels.append(op.rule)
                matcher.add(op.rule, [make_doc(vocab, words=pattern) for pattern in op.patterns])
                registered = [label for label in labels if label in matcher]
                if len(registered) != len(matcher):
                    raise ValueError('Rule count mismatch')
                patterns = [sorted(list(keyword) for keyword in stored_patterns(matcher, label)) for label in registered]
                states.append(LexicalState(None, registered, patterns, convert_matches(matcher(doc), vocab, len(doc))))
            cases.append(LexicalCase(f'{name}-{attribute.lower()}', attribute, input_words, spaces, operations, states))
    return LexicalFixture(1, versions, sources, cases)


class Options(argparse.Namespace):
    output: Path = Path('target/reports/phrase-reference.json')
    holdout: bool = False
    edges: bool = False
    annotations: bool = False
    lexical: bool = False


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--holdout', action='store_true')
    parser.add_argument('--edges', action='store_true')
    parser.add_argument('--annotations', action='store_true')
    parser.add_argument('--lexical', action='store_true')
    args = Options()
    parser.parse_args(namespace=args)
    output = args.output
    if output.exists():
        raise ValueError('Refusing to overwrite frozen output')
    fixture = build_lexical() if args.lexical else build_annotations() if args.annotations else build_edges() if args.edges else build(bool(args.holdout))
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(asdict(fixture), ensure_ascii=False, indent=2) + '\n')
    if isinstance(fixture, Fixture):
        print(f'{len(fixture.cases)} cases; {sum(len(c.states) for c in fixture.cases)} states; {sum(len(s.matches) for c in fixture.cases for s in c.states)} matches')
    elif isinstance(fixture, (AnnotationFixture, LexicalFixture)):
        print(f'{len(fixture.cases)} cases; {sum(len(c.states) for c in fixture.cases)} states; {sum(len(s.matches) for c in fixture.cases for s in c.states)} matches')
    else:
        print('1 explicit sentence-boundary case; 2 error-state probes')

if __name__ == '__main__':
    main()
