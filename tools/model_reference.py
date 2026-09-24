"""Freeze reference cases for contextual vectors and declared component order."""
import argparse
import json
from pathlib import Path
from typing import TypedDict
import spacy
import thinc
from evaluation_reference import EvaluationCase
from reference_types import float_values, token_records, span_records


class VectorCase(TypedDict):
    text: str
    tokens: list[list[float]]
    document: list[float]
    span: list[float]
    empty_span: list[float]


class Fixture(TypedDict):
    versions: dict[str, str]
    vector_model: str
    vector_cases: list[VectorCase]
    reordered_model: str
    reordered_pipeline: list[str]
    reordered: list[EvaluationCase]


def generate(output: Path) -> None:
    if output.exists():
        raise ValueError('Output exists; frozen expectations must not be overwritten')
    if (spacy.__version__, thinc.__version__) != ('3.8.14', '8.3.13'):
        raise ValueError('Use pinned reference versions')
    small = spacy.load('en_core_web_sm')
    vectors: list[VectorCase] = []
    for text in ['', 'hello', 'Alice visited London yesterday.', 'Zqxv café 🙂\n42']:
        doc = small(text)
        vectors.append({'text': text, 'tokens': [float_values(token.vector) for token in doc],
                        'document': float_values(doc.vector), 'span': float_values(doc[1:3].vector),
                        'empty_span': float_values(doc[0:0].vector)})
    medium = spacy.load('en_core_web_md')
    cases: list[EvaluationCase] = []
    for i, text in enumerate(['Alice left. Bob stayed in London.', 'Apple hired Alice. Microsoft hired Bob.', '']):
        doc = medium.tokenizer(text)
        for component in (medium.get_pipe('ner'), medium.get_pipe('tok2vec'), medium.get_pipe('tagger'),
                          medium.get_pipe('parser'), medium.get_pipe('attribute_ruler'), medium.get_pipe('lemmatizer')):
            doc = component(doc)
        cases.append({'id': f'ner-first-{i}', 'category': 'component-order', 'text': text,
                      'tokens': token_records(doc, text), 'entities': span_records(doc.ents),
                      'sentences': span_records(doc.sents), 'noun_chunks': span_records(doc.noun_chunks),
                      'vector': float_values(doc.vector)})
    fixture: Fixture = {'versions': {'spacy': spacy.__version__, 'thinc': thinc.__version__},
        'vector_model': 'en_core_web_sm 3.8.0', 'vector_cases': vectors,
        'reordered_model': 'en_core_web_md 3.8.0',
        'reordered_pipeline': ['ner', 'tok2vec', 'tagger', 'parser', 'attribute_ruler', 'lemmatizer'],
        'reordered': cases}
    output.parent.mkdir(parents=True, exist_ok=True)
    with output.open('x') as stream:
        json.dump(fixture, stream, ensure_ascii=False)
        stream.write('\n')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    path: object = parser.parse_args().output
    if not isinstance(path, Path):
        raise TypeError('Output must be a path')
    generate(path)
