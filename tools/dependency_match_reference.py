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
    IS_SUBSET: list[str]
    IS_SUPERSET: list[str]
    INTERSECTS: list[str]

# Numeric pattern operators, keyed exactly as spaCy spells them.
NumberOperator = TypedDict('NumberOperator', {
    '==': float, '!=': float, '>=': float, '<=': float, '>': float, '<': float,
    'IN': list[int], 'NOT_IN': list[int], 'IS_SUBSET': list[int], 'IS_SUPERSET': list[int], 'INTERSECTS': list[int],
}, total=False)
OfficialValue = str | bool | StringOperator | NumberOperator

class OfficialNode(TypedDict):
    RIGHT_ID: str
    RIGHT_ATTRS: dict[str, OfficialValue]
    LEFT_ID: NotRequired[str]
    REL_OP: NotRequired[str]

Attribute = Literal['text', 'lower', 'norm', 'lemma', 'pos', 'tag', 'dep', 'morphology',
                    'is_alpha', 'is_digit', 'is_space', 'is_punct', 'like_num', 'length',
                    'is_lower', 'is_upper', 'is_title', 'is_ascii', 'is_currency', 'is_stop',
                    'is_bracket', 'is_quote', 'is_left_punct', 'is_right_punct', 'like_url', 'like_email']
FLAG_ATTRIBUTES: tuple[Attribute, ...] = ('is_alpha', 'is_digit', 'is_space', 'is_punct', 'like_num')
MORE_FLAG_ATTRIBUTES: tuple[Attribute, ...] = ('is_lower', 'is_upper', 'is_title', 'is_ascii', 'is_currency', 'is_stop',
                                              'is_bracket', 'is_quote', 'is_left_punct', 'is_right_punct',
                                              'like_url', 'like_email')

@dataclass(frozen=True)
class Equals:
    value: str
    kind: Literal['equals'] = 'equals'

@dataclass(frozen=True)
class Membership:
    kind: Literal['in', 'not_in', 'morph_superset', 'morph_intersects', 'is_subset', 'is_superset', 'intersects']
    values: list[str]

@dataclass(frozen=True)
class Flag:
    value: bool
    kind: Literal['flag'] = 'flag'

Comparison = Literal['==', '!=', '>=', '<=', '>', '<']

@dataclass(frozen=True)
class Compare:
    operator: Comparison
    value: int | float
    kind: Literal['compare'] = 'compare'

@dataclass(frozen=True)
class IntegerMembership:
    kind: Literal['in_integers', 'not_in_integers', 'is_subset_integers', 'is_superset_integers', 'intersects_integers']
    values: list[int]

@dataclass(frozen=True)
class Constraint:
    attribute: Attribute
    predicate: Equals | Membership | Flag | Compare | IntegerMembership

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
              'is_alpha': 'IS_ALPHA', 'is_digit': 'IS_DIGIT', 'is_space': 'IS_SPACE', 'is_punct': 'IS_PUNCT', 'like_num': 'LIKE_NUM',
              'length': 'LENGTH',
              'is_lower': 'IS_LOWER', 'is_upper': 'IS_UPPER', 'is_title': 'IS_TITLE', 'is_ascii': 'IS_ASCII',
              'is_currency': 'IS_CURRENCY', 'is_stop': 'IS_STOP', 'is_bracket': 'IS_BRACKET', 'is_quote': 'IS_QUOTE',
              'is_left_punct': 'IS_LEFT_PUNCT', 'is_right_punct': 'IS_RIGHT_PUNCT', 'like_url': 'LIKE_URL',
              'like_email': 'LIKE_EMAIL'}


STRING_KEYS: dict[str, Literal['IN', 'NOT_IN', 'IS_SUBSET', 'IS_SUPERSET', 'INTERSECTS']] = {
    'in': 'IN', 'not_in': 'NOT_IN', 'morph_superset': 'IS_SUPERSET', 'morph_intersects': 'INTERSECTS',
    'is_subset': 'IS_SUBSET', 'is_superset': 'IS_SUPERSET', 'intersects': 'INTERSECTS'}
INTEGER_KEYS: dict[str, Literal['IN', 'NOT_IN', 'IS_SUBSET', 'IS_SUPERSET', 'INTERSECTS']] = {
    'in_integers': 'IN', 'not_in_integers': 'NOT_IN', 'is_subset_integers': 'IS_SUBSET',
    'is_superset_integers': 'IS_SUPERSET', 'intersects_integers': 'INTERSECTS'}


def official_attrs(constraints: list[Constraint]) -> dict[str, OfficialValue]:
    """Convert one item's conditions; operators on one attribute share one dict."""
    attrs: dict[str, OfficialValue] = {}
    numbers: dict[str, NumberOperator] = {}
    strings: dict[str, StringOperator] = {}
    for constraint in constraints:
        attribute = ATTRIBUTES[constraint.attribute]
        predicate = constraint.predicate
        if isinstance(predicate, Compare | IntegerMembership):
            if attribute in attrs or attribute in strings:
                raise ValueError('Official converter requires distinct attributes per node')
            operators = numbers.setdefault(attribute, NumberOperator())
            if isinstance(predicate, Compare):
                if predicate.operator in operators:
                    raise ValueError('Official converter requires distinct numeric operators')
                operators[predicate.operator] = predicate.value
            else:
                key = INTEGER_KEYS[predicate.kind]
                if key in operators:
                    raise ValueError('Official converter requires distinct numeric operators')
                operators[key] = predicate.values
            continue
        if isinstance(predicate, Membership):
            if attribute in attrs or attribute in numbers:
                raise ValueError('Official converter requires distinct attributes per node')
            string_operators = strings.setdefault(attribute, StringOperator())
            string_key = STRING_KEYS[predicate.kind]
            if string_key in string_operators:
                raise ValueError('Official converter requires distinct string operators')
            string_operators[string_key] = predicate.values
            continue
        if attribute in attrs or attribute in numbers or attribute in strings:
            raise ValueError('Official converter requires distinct attributes per node')
        attrs[attribute] = predicate.value
    attrs.update(numbers)
    attrs.update(strings)
    return attrs


def official(pattern: Pattern) -> list[OfficialNode]:
    result: list[OfficialNode] = []
    for node in pattern.nodes:
        attrs = official_attrs(node.constraints)
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


# LENGTH counts code points, like Python len(): decomposed accents, emoji sequences
# and supplementary characters show the difference from bytes and graphemes.
LENGTH_WORDS = ['a', 'ab', 'abc', 'abcd', '\u00e9', 'e\u0301', '\U0001f469\U0001f3fd\u200d\U0001f4bb', '\U00010400',
                'stra\u00dfe', '1,000', '\u03a3', 'supercalifragilistic', '\u00a0', '\n', "n't", '  ']
LENGTH_PIPELINE_TEXT = "The supercalifragilistic caf\u00e9 isn't 1,000 miles away \U0001f469\U0001f3fd\u200d\U0001f4bb."


def length_constraints() -> list[tuple[str, list[Constraint]]]:
    def compare(operator: Comparison, value: int | float) -> Constraint:
        return Constraint('length', Compare(operator, value))

    def members(kind: Literal['in_integers', 'not_in_integers'], values: list[int]) -> Constraint:
        return Constraint('length', IntegerMembership(kind, values))

    return [
        ('equals_3', [compare('==', 3)]), ('equals_3_float', [compare('==', 3.0)]), ('equals_0', [compare('==', 0)]),
        ('not_1', [compare('!=', 1)]), ('at_least_4', [compare('>=', 4)]), ('at_most_1', [compare('<=', 1)]),
        ('over_2_5', [compare('>', 2.5)]), ('under_2', [compare('<', 2)]), ('under_huge', [compare('<', 10**20)]),
        ('in_1_3', [members('in_integers', [1, 3])]), ('in_negative', [members('in_integers', [-1, 2])]),
        ('in_empty', [members('in_integers', [])]), ('in_large', [members('in_integers', [2**53 - 1])]),
        ('not_in_1_3', [members('not_in_integers', [1, 3])]), ('not_in_empty', [members('not_in_integers', [])]),
        ('between_2_4', [compare('>=', 2), compare('<', 4)]),
        ('in_not_2', [members('in_integers', [1, 2, 3]), compare('!=', 2)]),
        ('short_lower', [compare('<=', 3), Constraint('lower', Equals('abc'))]),
        ('not_2_5', [compare('!=', 2.5)]), ('not_3_float', [compare('!=', 3.0)]),
        ('equals_near_3', [compare('==', 3.0000000000000004)]), ('at_least_near_3', [compare('>=', 2.9999999999999996)]),
        ('over_negative', [compare('>', -1)]), ('in_duplicates', [members('in_integers', [3, 3])]),
        ('in_and_not_in', [members('in_integers', [1, 2, 3]), members('not_in_integers', [2])]),
        ('equals_2_5', [compare('==', 2.5)]), ('at_least_2_5', [compare('>=', 2.5)]),
        ('under_2_5', [compare('<', 2.5)]), ('at_most_negative', [compare('<=', -0.5)]),
        ('short_not_abc', [compare('<', 3), Constraint('lower', Equals('abc'))]),
        ('long_member', [compare('>=', 4), Constraint('lower', Membership('in', ['ab', 'abcd']))]),
    ]


def length_rules() -> list[Rule]:
    result = [Rule(name, [Pattern([Node('a', constraints)])]) for name, constraints in length_constraints()]
    result.append(Rule('long_head_short_child', [Pattern([
        Node('head', [Constraint('length', Compare('>=', 4))]),
        Node('child', [Constraint('length', Compare('<=', 2))], Link('head', '>')),
    ])]))
    return result


# Inputs for the remaining lexical flags: case (including titlecase letters and Greek),
# ASCII, currency symbols versus currency words, English stop words in every case,
# every bracket and quote spelling, and URL and email lookalikes.
MORE_FLAG_WORDS = ['hello', 'Hello', 'HELLO', 'hElLo', 'Hello-World', '\u01c5', '\u039f\u03a3', '\u03c3', '123', '\u00df',
                   '\u00e9', '\u00c9COLE', 'A1', 'ab1', '$', '\u20ac', '\u00a5', 'USD', '$5', '(', ')', '[', ']', '{', '}', '<', '>',
                   '"', "'", '`', '\u201a', '\u201b', '\u201e', '\u201f', '\u00ab', '\u00bb', '\u2018', '\u2019', '``', "''", '\u201c', '\u201d', '\u2039', '\u203a', '\u276e',
                   '\u276f', 'the', 'The', 'THE', 'whereby', 'also', 'https://spacy.io', 'http://x', 'www.example.com', 'www.',
                   'example.org', 'foo.bar', 'example.xyz', 'localhost:8080', 'example.com/', 'a@b.com',
                   'user.name+tag@example.co.uk', '@handle', '.com', 'file.txt', 'hello.', '1.5', '+1,234.00', 'U.S', '8.8.8.8',
                   '\u2019s', 'n\u2019t', '\u2018ll', '\u212aeep', 'X\u00aa', '\u2014', '\u2026']
MORE_FLAG_PIPELINE_TEXT = 'Email a@b.com or visit https://spacy.io \u2014 "The" (quoted) price is $5, NOT \u20ac4!'


def more_flag_constraints() -> list[tuple[str, list[Constraint]]]:
    single = [(f'{attribute}_{str(value).lower()}', [Constraint(attribute, Flag(value))])
              for attribute in MORE_FLAG_ATTRIBUTES for value in (True, False)]
    return single + [
        ('upper_stop', [Constraint('is_upper', Flag(True)), Constraint('is_stop', Flag(True))]),
        ('title_not_ascii', [Constraint('is_title', Flag(True)), Constraint('is_ascii', Flag(False))]),
        ('url_not_email', [Constraint('like_url', Flag(True)), Constraint('like_email', Flag(False))]),
        ('left_quote', [Constraint('is_quote', Flag(True)), Constraint('is_left_punct', Flag(True))]),
        ('stop_and_lower', [Constraint('is_stop', Flag(True)), Constraint('lower', Equals('the'))]),
    ]


def more_flag_rules() -> list[Rule]:
    result = [Rule(name, [Pattern([Node('a', constraints)])]) for name, constraints in more_flag_constraints()]
    result.append(Rule('title_head_stop_child', [Pattern([
        Node('head', [Constraint('is_title', Flag(True))]),
        Node('child', [Constraint('is_stop', Flag(True))], Link('head', '>')),
    ])]))
    return result


# One annotated document for set comparisons: case variants, multi-valued, repeated,
# empty and POS morphology, a combining accent, and Greek final sigma for LOWER.
SET_WORDS = ['The', 'cats', 'Who', 'were', 'NOT', 'here', '\u039f\u03a3', 'e\u0301', 'x', 'nom', 'noun']
SET_LEMMAS = ['the', 'cat', 'who', 'be', 'not', 'here', '\u03bf\u03c2', 'e\u0301', 'x', 'nom', 'noun']
SET_POS = ['DET', 'NOUN', 'PRON', 'AUX', 'PART', 'ADV', 'PROPN', 'X', 'X', 'NOUN', 'NOUN']
SET_TAGS = ['DT', 'NNS', 'WP', 'VBD', 'RB', 'RB', 'NNP', 'FW', 'FW', 'NN', 'NN']
SET_HEADS = [1, 3, 3, 3, 3, 3, 3, 6, 6, 6, 6]
SET_DEPS = ['det', 'nsubj', 'nsubj', 'ROOT', 'neg', 'advmod', 'npadvmod', 'dep', 'dep', 'dep', 'dep']
SET_MORPHS = ['Definite=Def|PronType=Art', 'Number=Plur', 'PronType=Int,Rel', 'Mood=Ind|Number=Plur|Tense=Past|VerbForm=Fin',
              'Polarity=Neg', 'PronType=Dem', 'Number=Sing', '', 'Foreign=Yes', 'Case=Nom,Nom', 'POS=NOUN']
SET_PIPELINE_TEXT = 'The cats were NOT here, but who knows?'
SET_KINDS: tuple[Literal['is_subset', 'is_superset', 'intersects'], ...] = ('is_subset', 'is_superset', 'intersects')
SET_VALUES: list[tuple[Attribute, list[list[str]]]] = [
    ('text', [[], ['The'], ['The', 'cats'], ['the'], ['The', 'The'], ['missing']]),
    ('lower', [[], ['the'], ['the', 'not'], ['The'], ['\u03bf\u03c2'], ['the', 'the']]),
    ('norm', [[], ['the'], ['were', 'not'], ['NOT']]),
    ('lemma', [[], ['be'], ['be', 'cat'], ['Be'], ['be', 'be']]),
    ('pos', [[], ['NOUN'], ['NOUN', 'PRON'], ['noun'], ['X', 'X']]),
    ('tag', [[], ['NNS'], ['NNS', 'WP', 'RB'], ['nns']]),
    ('dep', [[], ['nsubj'], ['nsubj', 'ROOT'], ['root'], ['dep', 'dep']]),
    ('morphology', [[], ['Number=Plur'], ['PronType=Int'], ['PronType=Int', 'PronType=Rel'], ['PronType=Int,Rel'],
                    ['PronType=Rel,Int'], ['Number=Plur', 'Number=Sing'], ['Definite=Def|PronType=Art'],
                    ['Definite=Def', 'PronType=Art'], ['pos=noun'], ['_'],
                    ['Mood=Ind', 'Number=Plur', 'Tense=Past', 'VerbForm=Fin', 'Extra=Yes'],
                    ['Number=Sing|Number=Plur'], ['Number=Plur|Number=Plur'], ['Case=Nom'], ['POS=NOUN', 'Number=Plur']]),
]
SET_LENGTHS: list[list[int]] = [[], [3], [1, 3], [3, 3], [-1], [2**53 - 1], [3, 4]]


def set_constraints() -> list[tuple[str, list[Constraint]]]:
    """Every set operator on every string attribute and on LENGTH, alone and combined."""
    result: list[tuple[str, list[Constraint]]] = []
    for attribute, value_lists in SET_VALUES:
        for kind in SET_KINDS:
            for index, values in enumerate(value_lists):
                result.append((f'{attribute}_{kind}_{index}', [Constraint(attribute, Membership(kind, values))]))
    integer_kinds: tuple[Literal['is_subset_integers', 'is_superset_integers', 'intersects_integers'], ...] = (
        'is_subset_integers', 'is_superset_integers', 'intersects_integers')
    for kind in integer_kinds:
        for index, values in enumerate(SET_LENGTHS):
            result.append((f'length_{kind}_{index}', [Constraint('length', IntegerMembership(kind, values))]))
    return result + [
        ('morph_superset_and_intersects', [Constraint('morphology', Membership('is_superset', ['Number=Plur'])),
                                           Constraint('morphology', Membership('intersects', ['Number=Plur', 'Tense=Past']))]),
        ('morph_subset_and_alias', [Constraint('morphology', Membership('is_subset', ['Number=Plur', 'Number=Sing', 'Polarity=Neg'])),
                                    Constraint('morphology', Membership('morph_intersects', ['Number=Plur']))]),
        ('lower_subset_length_superset', [Constraint('lower', Membership('is_subset', ['the', 'not', 'here'])),
                                          Constraint('length', IntegerMembership('is_superset_integers', [3]))]),
        ('length_subset_compare', [Constraint('length', IntegerMembership('is_subset_integers', [3, 4, 5])),
                                   Constraint('length', Compare('>', 3))]),
        ('length_in_and_intersects', [Constraint('length', IntegerMembership('in_integers', [1, 3])),
                                      Constraint('length', IntegerMembership('intersects_integers', [3, 4]))]),
        ('pos_in_and_superset', [Constraint('pos', Membership('in', ['NOUN', 'AUX'])),
                                 Constraint('pos', Membership('is_superset', ['NOUN']))]),
        ('lemma_subset_tag_intersects', [Constraint('lemma', Membership('is_subset', ['be', 'cat', 'who'])),
                                         Constraint('tag', Membership('intersects', ['VBD', 'WP']))]),
    ]


def set_rules() -> list[Rule]:
    result = [Rule(name, [Pattern([Node('a', constraints)])]) for name, constraints in set_constraints()]
    result.append(Rule('be_head_nominal_child', [Pattern([
        Node('head', [Constraint('lemma', Membership('is_subset', ['be']))]),
        Node('child', [Constraint('pos', Membership('intersects', ['NOUN', 'PRON']))], Link('head', '>')),
    ])]))
    return result


def set_document(nlp: Language) -> Doc:
    count = len(SET_WORDS)
    return make_doc(nlp.vocab, words=SET_WORDS, spaces=[index + 1 < count for index in range(count)], heads=SET_HEADS,
                    deps=SET_DEPS, pos=SET_POS, tags=SET_TAGS, lemmas=SET_LEMMAS, morphs=SET_MORPHS)


def lexeme_flag(nlp: Language, word: str, attribute: Attribute) -> bool:
    """Read a remaining lexical flag from the vocabulary, for independent checks."""
    lexeme = nlp.vocab[word]
    flags: dict[Attribute, bool] = {
        'is_lower': lexeme.is_lower, 'is_upper': lexeme.is_upper, 'is_title': lexeme.is_title,
        'is_ascii': lexeme.is_ascii, 'is_currency': lexeme.is_currency, 'is_stop': lexeme.is_stop,
        'is_bracket': lexeme.is_bracket, 'is_quote': lexeme.is_quote, 'is_left_punct': lexeme.is_left_punct,
        'is_right_punct': lexeme.is_right_punct, 'like_url': lexeme.like_url, 'like_email': lexeme.like_email,
    }
    return flags[attribute]


def attribute_name(value: str) -> Attribute:
    if value in ('text', 'lower', 'norm', 'lemma', 'pos', 'tag', 'dep', 'morphology'):
        return value
    for flag in FLAG_ATTRIBUTES + MORE_FLAG_ATTRIBUTES:
        if value == flag:
            return flag
    if value == 'length':
        return value
    raise ValueError('Unsupported attribute')


def case(nlp: Language, case_id: str, text: str, doc: Doc, patterns: list[Rule] | None = None) -> Case:
    if patterns is None:
        patterns = rules()
    matcher = DependencyMatcher(nlp.vocab, validate=True)
    for rule in patterns:
        matcher.add(rule.name, [official(pattern) for pattern in rule.patterns])
    expected = [Match(nlp.vocab.strings[key], indices) for key, indices in matcher(doc)]
    return Case(case_id, text, token_records(doc, text), span_records(doc.sents), [], [], patterns, expected)


def generate(regressions: bool = False, lower: bool = False, flags: bool = False, length: bool = False,
             more_flags: bool = False, sets: bool = False) -> Fixture:
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
    if length:
        return generate_length(nlp)
    if more_flags:
        return generate_more_flags(nlp)
    if sets:
        return generate_sets(nlp)
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


def generate_length(nlp: Language) -> Fixture:
    require_pinned_unicode()
    patterns = length_rules()
    count = len(LENGTH_WORDS)
    words = make_doc(nlp.vocab, words=LENGTH_WORDS, spaces=[index + 1 < count for index in range(count)],
                     heads=[0] * count, deps=['ROOT'] + ['dep'] * (count - 1))
    cases = [case(nlp, 'length-words', ' '.join(LENGTH_WORDS), words, patterns),
             case(nlp, 'length-pipeline', LENGTH_PIPELINE_TEXT, nlp(LENGTH_PIPELINE_TEXT), patterns),
             case(nlp, 'length-empty', '', make_doc(nlp.vocab, words=[]), patterns)]
    source = Path(spacy.__file__).parent / 'matcher' / 'dependencymatcher.pyx'
    return Fixture(Versions(spacy.__version__, thinc.__version__), 'en_core_web_md 3.8.0', hashlib.sha256(source.read_bytes()).hexdigest(), cases)


def generate_more_flags(nlp: Language) -> Fixture:
    require_pinned_unicode()
    patterns = more_flag_rules()
    count = len(MORE_FLAG_WORDS)
    words = make_doc(nlp.vocab, words=MORE_FLAG_WORDS, spaces=[index + 1 < count for index in range(count)],
                     heads=[0] * count, deps=['ROOT'] + ['dep'] * (count - 1))
    cases = [case(nlp, 'more-flag-words', ' '.join(MORE_FLAG_WORDS), words, patterns),
             case(nlp, 'more-flag-pipeline', MORE_FLAG_PIPELINE_TEXT, nlp(MORE_FLAG_PIPELINE_TEXT), patterns),
             case(nlp, 'more-flag-empty', '', make_doc(nlp.vocab, words=[]), patterns)]
    source = Path(spacy.__file__).parent / 'matcher' / 'dependencymatcher.pyx'
    return Fixture(Versions(spacy.__version__, thinc.__version__), 'en_core_web_md 3.8.0', hashlib.sha256(source.read_bytes()).hexdigest(), cases)


def generate_sets(nlp: Language) -> Fixture:
    require_pinned_unicode()
    patterns = set_rules()
    # Create documents before any pattern is registered: spaCy records the features of
    # a new morphology analysis as first spelled, so a pattern spelling such as
    # `pos=noun` registered first would change later documents' features.
    annotated, pipeline, empty = set_document(nlp), nlp(SET_PIPELINE_TEXT), make_doc(nlp.vocab, words=[])
    cases = [case(nlp, 'set-annotated', ' '.join(SET_WORDS), annotated, patterns),
             case(nlp, 'set-pipeline', SET_PIPELINE_TEXT, pipeline, patterns),
             case(nlp, 'set-empty', '', empty, patterns)]
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
    modes.add_argument('--length', action='store_true')
    modes.add_argument('--more-flags', action='store_true')
    modes.add_argument('--sets', action='store_true')
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
    length_mode: object = args.length
    if not isinstance(length_mode, bool):
        raise TypeError('Length flag must be boolean')
    more_flag_mode: object = args.more_flags
    if not isinstance(more_flag_mode, bool):
        raise TypeError('More-flags flag must be boolean')
    set_mode: object = args.sets
    if not isinstance(set_mode, bool):
        raise TypeError('Sets flag must be boolean')
    write_fixture(output, generate(regressions, lower_mode, flag_mode, length_mode, more_flag_mode, set_mode))


if __name__ == '__main__':
    main()
