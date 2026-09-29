"""Freeze native dependency-pattern expectations from official spaCy."""
import argparse
import hashlib
import json
import unicodedata
from dataclasses import asdict, dataclass
from pathlib import Path

# Typed boundary records mirror the narrow local spaCy matcher stub.
from typing import Literal, NotRequired, TypedDict

import spacy
import thinc
from reference_types import (
    Doc,
    Language,
    SpanRecord,
    TokenRecord,
    span_records,
    token_records,
)
from spacy.matcher import DependencyMatcher
from spacy.tokens import Doc as make_doc


class StringOperator(TypedDict, total=False):
    IN: list[str]
    NOT_IN: list[str]
    IS_SUPERSET: list[str]
    INTERSECTS: list[str]

class OfficialNode(TypedDict):
    RIGHT_ID: str
    RIGHT_ATTRS: dict[str, str | bool | StringOperator]
    LEFT_ID: NotRequired[str]
    REL_OP: NotRequired[str]

Attribute = Literal['text', 'lower', 'norm', 'lemma', 'pos', 'tag', 'dep', 'morphology',
                    'is_alpha', 'is_digit', 'is_space', 'is_punct', 'like_num']
FLAG_ATTRIBUTES: tuple[Attribute, ...] = ('is_alpha', 'is_digit', 'is_space', 'is_punct', 'like_num')

@dataclass(frozen=True)
class Equals:
    value: str
    kind: Literal['equals'] = 'equals'

@dataclass(frozen=True)
class Membership:
    kind: Literal['in', 'not_in', 'morph_superset', 'morph_intersects']
    values: list[str]

@dataclass(frozen=True)
class Flag:
    value: bool
    kind: Literal['flag'] = 'flag'

@dataclass(frozen=True)
class Constraint:
    attribute: Attribute
    predicate: Equals | Membership | Flag

@dataclass(frozen=True)
class Link:
    left: str
    relation: str

@dataclass(frozen=True)
class Node:
    id: str
    constraints: list[Constraint]
    link: Link | None = None

@dataclass(frozen=True)
class Pattern:
    nodes: list[Node]

@dataclass(frozen=True)
class Rule:
    name: str
    patterns: list[Pattern]

@dataclass(frozen=True)
class Match:
    rule: str
    tokens: list[int]

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
class Versions:
    spacy: str
    thinc: str

@dataclass(frozen=True)
class Fixture:
    versions: Versions
    model: str
    source_sha256: str
    cases: list[Case]

OPERATORS = ('<', '>', '<<', '>>', '.', '.*', ';', ';*', '$+', '$-', '$++', '$--', '>+', '>-', '>++', '>--', '<+', '<-', '<++', '<--')
ATTRIBUTES = {'text': 'ORTH', 'lower': 'LOWER', 'norm': 'NORM', 'lemma': 'LEMMA', 'pos': 'POS', 'tag': 'TAG', 'dep': 'DEP', 'morphology': 'MORPH',
              'is_alpha': 'IS_ALPHA', 'is_digit': 'IS_DIGIT', 'is_space': 'IS_SPACE', 'is_punct': 'IS_PUNCT', 'like_num': 'LIKE_NUM'}


def official(pattern: Pattern) -> list[OfficialNode]:
    result: list[OfficialNode] = []
    for node in pattern.nodes:
        attrs: dict[str, str | bool | StringOperator] = {}
        for constraint in node.constraints:
            if ATTRIBUTES[constraint.attribute] in attrs:
                raise ValueError('Official converter requires distinct attributes per node')
            predicate = constraint.predicate
            if isinstance(predicate, Flag):
                value: str | bool | StringOperator = predicate.value
            elif isinstance(predicate, Equals):
                value = predicate.value
            elif predicate.kind == 'in':
                value = {'IN': predicate.values}
            elif predicate.kind == 'not_in':
                value = {'NOT_IN': predicate.values}
            elif predicate.kind == 'morph_superset':
                value = {'IS_SUPERSET': predicate.values}
            else:
                value = {'INTERSECTS': predicate.values}
            attrs[ATTRIBUTES[constraint.attribute]] = value
        converted: OfficialNode = {'RIGHT_ID': node.id, 'RIGHT_ATTRS': attrs}
        if node.link is not None:
            converted['LEFT_ID'] = node.link.left
            converted['REL_OP'] = node.link.relation
        result.append(converted)
    return result


def rules() -> list[Rule]:
    result = [Rule(op, [Pattern([Node('a', []), Node('b', [], Link('a', op))])]) for op in OPERATORS]
    same = Pattern([Node('a', []), Node('b', [], Link('a', '>')), Node('c', [], Link('a', '>'))])
    result.extend([Rule('branch_reuse', [same, same]), Rule('branch_reuse', [same])])
    for attr, val in [('text', 'Alice'), ('norm', 'alice'), ('lemma', 'see'), ('pos', 'VERB'), ('tag', 'NNP'), ('dep', 'nsubj'), ('morphology', 'Number=Sing')]:
        # Explicit typed records avoid accepting arbitrary attribute identifiers.
        attribute = attribute_name(attr)
        result.append(Rule('equals_' + attr, [Pattern([Node('a', [Constraint(attribute, Equals(val))])])]))
        result.append(Rule('in_' + attr, [Pattern([Node('a', [Constraint(attribute, Membership('in', [val]))])])]))
        result.append(Rule('not_in_' + attr, [Pattern([Node('a', [Constraint(attribute, Membership('not_in', [val]))])])]))
    for kind in ('morph_superset', 'morph_intersects'):
        result.append(Rule(kind, [Pattern([Node('a', [Constraint('morphology', Membership(kind, ['Number=Sing', 'Tense=Past']))])])]))
    for name, predicate in [
        ('morph_noncanonical_equals', Equals('Number=Sing|Case=Nom,Acc')),
        ('morph_noncanonical_in', Membership('in', ['Number=Sing|Case=Nom,Acc'])),
        ('morph_single_superset', Membership('morph_superset', ['Case=Nom'])),
        ('morph_combined_superset', Membership('morph_superset', ['Case=Acc,Nom'])),
        ('morph_empty_in', Membership('in', [''])),
        ('morph_underscore_in', Membership('in', ['_'])),
    ]:
        result.append(Rule(name, [Pattern([Node('a', [Constraint('morphology', predicate)])])]))
    result.append(Rule('empty_set', [Pattern([Node('a', [Constraint('text', Membership('in', []))])])]))
    return result


# Python str.lower() cases: final sigma, combining marks, expansions, supplementary
# characters, titlecase, the Kelvin sign, a ligature without a lowercase mapping, and
# emoji. U+1C89 is unassigned in the pinned Unicode 15.0.0 data but has a lowercase
# mapping in later Unicode versions, so it detects use of an unpinned lowercase table.
LOWER_WORDS = ['The', 'THE', 'the', 'Ritz', 'RITZ', 'ritz', 'ΟΣ', 'ΟΣΑ', 'Σ', 'σ', 'AΣ\u0301', 'İ', 'I',
               'ẞ', 'ß', 'SS', 'É', 'e\u0301', '𐐀', 'K', 'ﬀ', 'ǅ', 'Straße', '123', '👩🏽\u200d💻', '\u212a', '\u1c89']
LOWER_VALUES = ['the', 'THE', 'ritz', 'ος', 'οσ', 'οσα', 'σ', 'ς', 'aς\u0301', 'i\u0307', 'i', 'ß', 'ss', 'é',
                'e\u0301', '𐐨', 'k', 'ﬀ', 'ff', 'ǆ', 'straße', '123', '👩🏽\u200d💻', '', '\u1c89', '\u1c8a', 'gon', 'going']
# Uppercase set values must never match: pattern values are compared as written.
LOWER_SETS: list[tuple[str, list[str]]] = [('articles', ['the', 'ritz']), ('unicode', ['ος', 'ß', 'i\u0307']), ('empty', []),
                                           ('uppercase', ['THE', 'ΟΣ'])]
# Tokenizer exceptions give `Gon`/`na` norms `going`/`to`, separating LOWER from NORM.
LOWER_PIPELINE_TEXT = 'THE Ritz welcomed ΟΣ guests at the RITZ. Gonna'


def require_pinned_unicode() -> None:
    if unicodedata.unidata_version != '15.0.0':
        raise ValueError('LOWER references require the pinned Python Unicode 15.0.0 data')


def lower_constraints() -> list[tuple[str, Constraint]]:
    result = [(f'equals_{index}', Constraint('lower', Equals(value))) for index, value in enumerate(LOWER_VALUES)]
    for name, values in LOWER_SETS:
        result.append((f'in_{name}', Constraint('lower', Membership('in', values))))
        result.append((f'not_in_{name}', Constraint('lower', Membership('not_in', values))))
    return result


def lower_rules() -> list[Rule]:
    result = [Rule(name, [Pattern([Node('a', [constraint])])]) for name, constraint in lower_constraints()]
    result.extend([
        Rule('child_the', [Pattern([Node('head', [Constraint('lower', Equals('ritz'))]),
                                    Node('child', [Constraint('lower', Equals('the'))], Link('head', '>'))])]),
        Rule('subject_set', [Pattern([Node('verb', [Constraint('lower', Equals('welcomed'))]),
                                      Node('arg', [Constraint('lower', Membership('in', ['ritz', 'guests']))], Link('verb', '>'))])]),
        Rule('conjunction', [Pattern([Node('a', [Constraint('lower', Equals('the')), Constraint('text', Equals('THE'))])])]),
        Rule('precedes_sigma', [Pattern([Node('a', [Constraint('lower', Equals('ritz'))]),
                                         Node('b', [Constraint('lower', Equals('ος'))], Link('a', '.*'))])]),
    ])
    return result


# Lexical flag inputs: Unicode letters, digits and punctuation categories, whitespace,
# signed, grouped and fractional numbers, ordinals, and English number words, which
# spaCy's English LIKE_NUM adds to the language-independent rule.
FLAG_WORDS = ['Hello', 'hello123', '123', '\u0661\u0662\u0663', '\u00b2', '\u216b', '\u00bd', '...', '\u2014', '$',
              '\u00bf', '\u00ab', '@', '+', '_', '-5', '+1,000.5', '1/2', '1/2/3', '3rd', '21st', '11th', '3th',
              'ten', 'TEN', 'Million', 'first', 'Tenth', 'twenty-one', 'Σ', 'e\u0301', '\u00a0', '\n', '\U0001f642',
              "n't", '1.5e3', '\u0663rd', '\u00b15', '~5', '--5', '3RD', 'st', '.', 'Twelfth', '\u2026', '--',
              '\u0085', '\u001c']
FLAG_PIPELINE_TEXT = 'I paid $1,000.50 for twenty-one tickets on the 3rd \u2014 ten were free!'


def flag_constraints() -> list[tuple[str, Constraint]]:
    return [(f'{attribute}_{str(value).lower()}', Constraint(attribute, Flag(value)))
            for attribute in FLAG_ATTRIBUTES for value in (True, False)]


def flag_rules() -> list[Rule]:
    result = [Rule(name, [Pattern([Node('a', [constraint])])]) for name, constraint in flag_constraints()]
    result.extend([
        Rule('alpha_hello', [Pattern([Node('a', [Constraint('is_alpha', Flag(True)), Constraint('lower', Equals('hello'))])])]),
        Rule('number_not_alpha', [Pattern([Node('a', [Constraint('is_alpha', Flag(False)), Constraint('like_num', Flag(True))])])]),
        Rule('number_word', [Pattern([Node('a', [Constraint('is_alpha', Flag(True)), Constraint('like_num', Flag(True))])])]),
        Rule('word_with_number_child', [Pattern([Node('head', [Constraint('is_alpha', Flag(True))]),
                                                 Node('number', [Constraint('like_num', Flag(True))], Link('head', '>'))])]),
    ])
    return result


def attribute_name(value: str) -> Attribute:
    if value in ('text', 'lower', 'norm', 'lemma', 'pos', 'tag', 'dep', 'morphology'):
        return value
    for flag in FLAG_ATTRIBUTES:
        if value == flag:
            return flag
    raise ValueError('Unsupported attribute')


def case(nlp: Language, case_id: str, text: str, doc: Doc, patterns: list[Rule] | None = None) -> Case:
    if patterns is None:
        patterns = rules()
    matcher = DependencyMatcher(nlp.vocab, validate=True)
    for rule in patterns:
        matcher.add(rule.name, [official(pattern) for pattern in rule.patterns])
    expected = [Match(nlp.vocab.strings[key], indices) for key, indices in matcher(doc)]
    return Case(case_id, text, token_records(doc, text), span_records(doc.sents), [], [], patterns, expected)


def generate(regressions: bool = False, lower: bool = False, flags: bool = False) -> Fixture:
    if (spacy.__version__, thinc.__version__) != ('3.8.14', '8.3.13'):
        raise ValueError('Use the pinned reference environment')
    nlp = spacy.load('en_core_web_md')
    if nlp.meta['version'] != '3.8.0':
        raise ValueError('Use en_core_web_md 3.8.0')
    if regressions:
        return generate_regressions(nlp)
    if lower:
        return generate_lower(nlp)
    if flags:
        return generate_flags(nlp)
    cases: list[Case] = []
    for case_id, text in [('ordinary', 'Alice saw Bob and Carol.'), ('sentences', 'Alice left. Bob stayed.'), ('unicode', 'Zoë sees 👩🏽‍💻 today.'), ('empty', '')]:
        cases.append(case(nlp, case_id, text, nlp(text)))
    words = ['Alice', 'saw', 'Bob', 'near', 'Carol', 'today']
    doc = make_doc(nlp.vocab, words=words, spaces=[True] * 5 + [False], heads=[2, 1, 1, 1, 2, 3], deps=['nsubj', 'ROOT', 'dobj', 'prep', 'conj', 'pobj'], pos=['PROPN', 'VERB', 'PROPN', 'ADP', 'PROPN', 'NOUN'], tags=['NNP', 'VBD', 'NNP', 'IN', 'NNP', 'NN'], lemmas=['Alice', 'see', 'Bob', 'near', 'Carol', 'today'], morphs=['Number=Sing', 'Tense=Past|VerbForm=Fin', 'Number=Sing', '', 'Number=Sing', 'Number=Sing'])
    cases.append(case(nlp, 'crossing', ' '.join(words), doc))
    morph_doc = make_doc(nlp.vocab, words=['case', 'empty'], spaces=[True, False], heads=[0, 0], deps=['ROOT', 'dep'], pos=['NOUN', 'NOUN'], tags=['NN', 'NN'], lemmas=['case', 'empty'], morphs=['Case=Acc,Nom|Number=Sing', ''])
    cases.append(case(nlp, 'morphology', 'case empty', morph_doc))
    source = Path(spacy.__file__).parent / 'matcher' / 'dependencymatcher.pyx'
    return Fixture(Versions(spacy.__version__, thinc.__version__), 'en_core_web_md 3.8.0', hashlib.sha256(source.read_bytes()).hexdigest(), cases)


def generate_regressions(nlp: Language) -> Fixture:
    predicates: list[tuple[str, Equals | Membership]] = [
        ('empty_equals', Equals('')),
        ('underscore_equals', Equals('_')),
        ('duplicate_field', Membership('in', ['Case=Acc|Case=Nom'])),
        ('duplicate_value', Membership('in', ['Case=Nom,Nom'])),
        ('pos_lower', Membership('in', ['pos=noun'])),
        ('pos_upper', Membership('in', ['POS=NOUN'])),
    ]
    patterns = [Rule(name, [Pattern([Node('a', [Constraint('morphology', predicate)])])]) for name, predicate in predicates]
    words = ['empty', 'nom', 'duplicate', 'pos']
    doc = make_doc(nlp.vocab, words=words, spaces=[True, True, True, False], heads=[0, 0, 0, 0], deps=['ROOT', 'dep', 'dep', 'dep'], pos=['NOUN'] * 4, tags=['NN'] * 4, lemmas=words, morphs=['', 'Case=Nom', 'Case=Nom,Nom', 'POS=NOUN'])
    cases = [case(nlp, 'morphology-aliases', ' '.join(words), doc, patterns)]
    words = ['match', 'match', 'skip', 'skip']
    doc = make_doc(nlp.vocab, words=words, spaces=[True, True, True, False], heads=[3, 1, 1, 3], deps=['dep', 'ROOT', 'dep', 'ROOT'], pos=['NOUN'] * 4, tags=['NN'] * 4, lemmas=['keep', 'keep', 'skip', 'skip'], morphs=[''] * 4)
    selected = [Constraint('text', Equals('match')), Constraint('lemma', Equals('keep'))]
    patterns = [Rule('selective_roots', [Pattern([Node('a', selected)])])]
    cases.append(case(nlp, 'selective-root-order', ' '.join(words), doc, patterns))
    source = Path(spacy.__file__).parent / 'matcher' / 'dependencymatcher.pyx'
    return Fixture(Versions(spacy.__version__, thinc.__version__), 'en_core_web_md 3.8.0', hashlib.sha256(source.read_bytes()).hexdigest(), cases)


def generate_lower(nlp: Language) -> Fixture:
    require_pinned_unicode()
    patterns = lower_rules()
    cases: list[Case] = []
    count = len(LOWER_WORDS)
    variants = make_doc(nlp.vocab, words=LOWER_WORDS, spaces=[index + 1 < count for index in range(count)],
                        heads=[0] * count, deps=['ROOT'] + ['dep'] * (count - 1))
    cases.append(case(nlp, 'lower-variants', ' '.join(LOWER_WORDS), variants, patterns))
    words = ['THE', 'RITZ', 'welcomed', 'ΟΣ', 'Guests']
    tree = make_doc(nlp.vocab, words=words, spaces=[True] * 4 + [False], heads=[1, 2, 2, 4, 2],
                    deps=['det', 'nsubj', 'ROOT', 'amod', 'dobj'])
    cases.append(case(nlp, 'lower-tree', ' '.join(words), tree, patterns))
    cases.append(case(nlp, 'lower-pipeline', LOWER_PIPELINE_TEXT, nlp(LOWER_PIPELINE_TEXT), patterns))
    empty = make_doc(nlp.vocab, words=[])
    cases.append(case(nlp, 'lower-empty', '', empty, patterns))
    source = Path(spacy.__file__).parent / 'matcher' / 'dependencymatcher.pyx'
    return Fixture(Versions(spacy.__version__, thinc.__version__), 'en_core_web_md 3.8.0', hashlib.sha256(source.read_bytes()).hexdigest(), cases)


def generate_flags(nlp: Language) -> Fixture:
    require_pinned_unicode()
    patterns = flag_rules()
    count = len(FLAG_WORDS)
    words = make_doc(nlp.vocab, words=FLAG_WORDS, spaces=[index + 1 < count for index in range(count)],
                     heads=[0] * count, deps=['ROOT'] + ['dep'] * (count - 1))
    cases = [case(nlp, 'flag-words', ' '.join(FLAG_WORDS), words, patterns),
             case(nlp, 'flag-pipeline', FLAG_PIPELINE_TEXT, nlp(FLAG_PIPELINE_TEXT), patterns),
             case(nlp, 'flag-empty', '', make_doc(nlp.vocab, words=[]), patterns)]
    source = Path(spacy.__file__).parent / 'matcher' / 'dependencymatcher.pyx'
    return Fixture(Versions(spacy.__version__, thinc.__version__), 'en_core_web_md 3.8.0', hashlib.sha256(source.read_bytes()).hexdigest(), cases)


def write_fixture(output: Path, fixture: Fixture) -> None:
    output.parent.mkdir(parents=True, exist_ok=True)
    with output.open('x', encoding='utf-8') as stream:
        stream.write(json.dumps(asdict(fixture), ensure_ascii=False) + '\n')


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    modes = parser.add_mutually_exclusive_group()
    modes.add_argument('--regressions', action='store_true')
    modes.add_argument('--lower', action='store_true')
    modes.add_argument('--flags', action='store_true')
    args = parser.parse_args()
    output: object = args.output
    if not isinstance(output, Path):
        raise TypeError('Output must be a path')
    if output.exists():
        parser.error('Output exists; frozen expectations stay unchanged')
    regressions: object = args.regressions
    if not isinstance(regressions, bool):
        raise TypeError('Regressions flag must be boolean')
    lower_mode: object = args.lower
    if not isinstance(lower_mode, bool):
        raise TypeError('Lower flag must be boolean')
    flag_mode: object = args.flags
    if not isinstance(flag_mode, bool):
        raise TypeError('Flags flag must be boolean')
    write_fixture(output, generate(regressions, lower_mode, flag_mode))


if __name__ == '__main__':
    main()
