"""Regenerate pinned model-independent LOWER resources and phrase comparisons."""
import argparse
from dataclasses import asdict, dataclass
import hashlib
import importlib.metadata
import json
from pathlib import Path
import unicodedata

import spacy
from spacy.matcher import PhraseMatcher
from spacy.tokens import Doc as make_doc
from lexical import export
from phrase_match_reference import Case, Operation, State, convert_matches, verify_sources
from json_types import json_object, json_string, read_json

ROOT = Path(__file__).resolve().parent.parent
RESOURCE = ROOT / 'crates/spars/src/unicode_lower.json'
FIXTURE = ROOT / 'fixtures/phrase-match-lower-v1.expected.json'

@dataclass(frozen=True)
class LowerResource:
    unicode_version: str
    lower: dict[str, str]
    cased: list[list[int]]
    case_ignorable: list[list[int]]

@dataclass(frozen=True)
class LowerFixture:
    format_version: int
    versions: dict[str, str]
    sources: dict[str, str]
    cases: list[Case]


def build() -> tuple[LowerResource, LowerFixture]:
    versions, sources = verify_sources()
    if unicodedata.unidata_version != '15.0.0':
        raise ValueError('LOWER requires the pinned Python Unicode 15.0.0 data')
    source = 'spacy/lang/lex_attrs.py'
    lock = json_object(json_object(read_json(ROOT / 'reference/source-lock.json'))['spacy'])
    expected = json_string(json_object(lock['files'])[source])
    actual = hashlib.sha256(Path(str(importlib.metadata.distribution('spacy').locate_file(source))).read_bytes()).hexdigest()
    if actual != expected:
        raise ValueError('Pinned lexical source mismatch')
    sources[source] = actual
    versions['unicode'] = unicodedata.unidata_version
    nlp = spacy.blank('en')
    lexical = export(nlp)
    ranges: list[list[int]] = []
    for point in range(0x110000):
        char = chr(point)
        if char.islower() or char.isupper() or unicodedata.category(char) == 'Lt':
            if ranges and ranges[-1][1] == point - 1:
                ranges[-1][1] = point
            else:
                ranges.append([point, point])
    resource = LowerResource(lexical['unicode_version'], lexical['lower'], ranges, lexical['ranges']['case_ignorable'])
    cases: list[Case] = []
    samples: list[tuple[str, list[str], list[list[str]]]] = [
        ('ritz', ['The', 'Ritz', 'the', 'ritz', 'THE', 'RITZ'], [['the', 'ritz'], ['RITZ']]),
        ('sigma', ['ΟΣ', 'ος', 'οσ', 'ΟΣΑ', 'οσα', 'Σ', 'σ', 'AΣ\u0301', 'aς\u0301', 'AΣ\u0301A', 'aσ\u0301a'], [['ος'], ['οσ'], ['οσα'], ['σ'], ['aς\u0301'], ['aσ\u0301a']]),
        ('unicode', ['İ', 'i\u0307', 'I', 'i', 'ẞ', 'ß', 'SS', 'ss', 'É', 'é', 'e\u0301', '𐐀', '𐐨', 'K', 'k', 'ﬀ', 'ff'], [['i\u0307'], ['i'], ['ß'], ['ss'], ['é'], ['e\u0301'], ['𐐨'], ['k'], ['ff']]),
        ('overlap', ['A', 'a', 'A'], [['a'], ['a', 'A'], ['A', 'a', 'A']]),
    ]
    for name, words, patterns in samples:
        matcher = PhraseMatcher(nlp.vocab, attr='LOWER')
        doc = make_doc(nlp.vocab, words=words)
        operations = [Operation('add', 'first', patterns), Operation('add', 'second', patterns), Operation('add', 'first', patterns), Operation('remove', 'first', []), Operation('add', 'first', patterns), Operation('remove', 'second', [])]
        states: list[State] = []
        for operation in operations:
            if operation.action == 'add':
                matcher.add(operation.rule, [make_doc(nlp.vocab, words=pattern) for pattern in operation.patterns])
            else:
                matcher.remove(operation.rule)
            states.append(State(None, [rule for rule in ['first', 'second'] if rule in matcher], convert_matches(matcher(doc), nlp.vocab, len(doc))))
        cases.append(Case(name, words, [True] * len(words), operations, states))
    return resource, LowerFixture(1, versions, sources, cases)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    class Options(argparse.Namespace):
        check: bool = False
    options = Options()
    parser.parse_args(namespace=options)
    resource, fixture = build()
    for path, data in [(RESOURCE, asdict(resource)), (FIXTURE, asdict(fixture))]:
        content = json.dumps(data, ensure_ascii=False, indent=2) + '\n'
        if options.check:
            if path.read_text() != content:
                raise ValueError(f'Regenerated content differs: {path.relative_to(ROOT)}')
        else:
            if path.exists():
                raise ValueError(f'Refusing to overwrite {path.relative_to(ROOT)}')
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(content)
        print(f'{path.relative_to(ROOT)} sha256={hashlib.sha256(content.encode()).hexdigest()}')

if __name__ == '__main__':
    main()
