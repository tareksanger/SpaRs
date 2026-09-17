"""Freeze dependency traversal results from the pinned official English model."""
import argparse
from dataclasses import asdict, dataclass
import hashlib
import json
from pathlib import Path

import spacy
import thinc

from json_types import json_array, json_int, json_object, json_string, parse_json
from reference_types import Token


@dataclass(frozen=True)
class Versions:
    spacy: str
    thinc: str


@dataclass(frozen=True)
class InputCase:
    id: str
    text: str
    heads: list[int]


@dataclass(frozen=True)
class TraversalToken:
    index: int
    children: list[int]
    ancestors: list[int]
    subtree: list[int]
    sentence: tuple[int, int]


@dataclass(frozen=True)
class TraversalCase:
    id: str
    text: str
    tokens: list[TraversalToken]


@dataclass(frozen=True)
class TraversalFixture:
    versions: Versions
    model: str
    input_sha256: str
    cases: list[TraversalCase]


def input_cases(raw: bytes) -> list[InputCase]:
    corpus = json_object(parse_json(raw.decode('utf-8')))
    versions = json_object(corpus['versions'])
    if versions != {'spacy': '3.8.14', 'thinc': '8.3.13'} or corpus['model'] != 'en_core_web_md 3.8.0':
        raise ValueError('Input must use the pinned model and reference versions')
    result: list[InputCase] = []
    seen: set[str] = set()
    for value in json_array(corpus['cases']):
        item = json_object(value)
        case_id = json_string(item['id'])
        if not case_id or case_id in seen:
            raise ValueError('Case identifiers must be nonempty and unique')
        seen.add(case_id)
        heads = [json_int(json_object(token)['head']) for token in json_array(item['tokens'])]
        result.append(InputCase(case_id, json_string(item['text']), heads))
    if not result:
        raise ValueError('Input must contain at least one case')
    return result


def token_traversal(token: Token) -> TraversalToken:
    sentence = token.sent
    return TraversalToken(token.i, [child.i for child in token.children],
                          [ancestor.i for ancestor in token.ancestors],
                          [descendant.i for descendant in token.subtree],
                          (sentence.start, sentence.end))


def write_fixture(output: Path, fixture: TraversalFixture) -> None:
    output.parent.mkdir(parents=True, exist_ok=True)
    with output.open('x', encoding='utf-8') as stream:
        stream.write(json.dumps(asdict(fixture), ensure_ascii=False) + '\n')


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('input', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    input_path: object = args.input
    output_path: object = args.output
    if not isinstance(input_path, Path) or not isinstance(output_path, Path):
        raise TypeError('Input and output must be paths')
    if output_path.exists():
        parser.error('Output exists. Use a new version; frozen expectations stay unchanged.')
    if (spacy.__version__, thinc.__version__) != ('3.8.14', '8.3.13'):
        parser.error('Use the pinned reference environment.')
    raw = input_path.read_bytes()
    inputs = input_cases(raw)
    model = spacy.load('en_core_web_md')
    if model.meta['version'] != '3.8.0' or model.meta['lang'] != 'en' or model.meta['name'] != 'core_web_md':
        parser.error('Use the official en_core_web_md 3.8.0 model.')
    cases: list[TraversalCase] = []
    for case in inputs:
        doc = model(case.text)
        if [token.head.i for token in doc] != case.heads:
            raise ValueError(f'Official dependency heads differ from frozen input: {case.id}')
        cases.append(TraversalCase(case.id, case.text, [token_traversal(token) for token in doc]))
    fixture = TraversalFixture(Versions(spacy.__version__, thinc.__version__),
                               'en_core_web_md 3.8.0', hashlib.sha256(raw).hexdigest(), cases)
    write_fixture(output_path, fixture)
    print(f'Wrote {len(cases)} cases, {sum(len(case.tokens) for case in cases)} tokens')


if __name__ == '__main__':
    main()
