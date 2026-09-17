"""Freeze native dependency-pattern expectations from official spaCy."""
import argparse
from dataclasses import asdict, dataclass
import hashlib
import json
from pathlib import Path
from typing import Literal

import spacy
import thinc
from spacy.matcher import DependencyMatcher
from spacy.tokens import Doc as make_doc

from reference_types import Doc, Language, SpanRecord, TokenRecord, span_records, token_records

# Typed boundary records mirror the narrow local spaCy matcher stub.
from typing import TypedDict, NotRequired

class StringOperator(TypedDict, total=False):
    IN: list[str]
    NOT_IN: list[str]
    IS_SUPERSET: list[str]
    INTERSECTS: list[str]

class OfficialNode(TypedDict):
    RIGHT_ID: str
    RIGHT_ATTRS: dict[str, str | StringOperator]
    LEFT_ID: NotRequired[str]
    REL_OP: NotRequired[str]

Attribute = Literal['text', 'norm', 'lemma', 'pos', 'tag', 'dep', 'morphology']

@dataclass(frozen=True)
class Equals:
    value: str
    kind: Literal['equals'] = 'equals'

@dataclass(frozen=True)
class Membership:
    kind: Literal['in', 'not_in', 'morph_superset', 'morph_intersects']
    values: list[str]

@dataclass(frozen=True)
class Constraint:
    attribute: Attribute
    predicate: Equals | Membership

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
ATTRIBUTES = {'text': 'ORTH', 'norm': 'NORM', 'lemma': 'LEMMA', 'pos': 'POS', 'tag': 'TAG', 'dep': 'DEP', 'morphology': 'MORPH'}


def official(pattern: Pattern) -> list[OfficialNode]:
    result: list[OfficialNode] = []
    for node in pattern.nodes:
        attrs: dict[str, str | StringOperator] = {}
        for constraint in node.constraints:
            if ATTRIBUTES[constraint.attribute] in attrs:
                raise ValueError('Official converter requires distinct attributes per node')
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


def attribute_name(value: str) -> Attribute:
    if value in ('text', 'norm', 'lemma', 'pos', 'tag', 'dep', 'morphology'):
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


def generate(regressions: bool = False) -> Fixture:
    if (spacy.__version__, thinc.__version__) != ('3.8.14', '8.3.13'):
        raise ValueError('Use the pinned reference environment')
    nlp = spacy.load('en_core_web_md')
    if nlp.meta['version'] != '3.8.0':
        raise ValueError('Use en_core_web_md 3.8.0')
    if regressions:
        return generate_regressions(nlp)
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


def write_fixture(output: Path, fixture: Fixture) -> None:
    output.parent.mkdir(parents=True, exist_ok=True)
    with output.open('x', encoding='utf-8') as stream:
        stream.write(json.dumps(asdict(fixture), ensure_ascii=False) + '\n')


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--regressions', action='store_true')
    args = parser.parse_args()
    output: object = args.output
    if not isinstance(output, Path):
        raise TypeError('Output must be a path')
    if output.exists():
        parser.error('Output exists; frozen expectations stay unchanged')
    regressions: object = args.regressions
    if not isinstance(regressions, bool):
        raise TypeError('Regressions flag must be boolean')
    write_fixture(output, generate(regressions))


if __name__ == '__main__':
    main()
