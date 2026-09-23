"""Create a new official-reference fixture without overwriting an existing one."""
import argparse
import hashlib
import json
from pathlib import Path

import spacy
import thinc


from reference_types import TokenRecord, SpanRecord, token_records, span_records, float_values
from typing import TypedDict
from model_catalog import release
from json_types import json_object, json_array, json_string, parse_json


class EvaluationCase(TypedDict):
    id: str
    category: str
    text: str
    tokens: list[TokenRecord]
    entities: list[SpanRecord]
    sentences: list[SpanRecord]
    noun_chunks: list[SpanRecord]
    vector: list[float]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--model', default='en_core_web_md')
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
    model_name: object = args.model
    if not isinstance(model_name, str):
        raise TypeError('Model must be a string')
    entry = release(model_name)
    model = spacy.load(entry.model)
    assert model.meta['version'] == '3.8.0'
    raw = input_path.read_bytes()
    corpus = json_object(parse_json(raw.decode()))
    cases: list[EvaluationCase] = []
    token_count = 0
    for item in json_array(corpus['cases']):
        case = json_object(item)
        text = json_string(case['text'])
        doc = model(text)
        token_count += len(doc)
        if set(case) != {'id', 'category', 'text'}:
            raise ValueError('Each input case must contain id, category, and text')
        cases.append({'id': json_string(case['id']), 'category': json_string(case['category']), 'text': text, 'tokens': token_records(doc, text),
            'entities': span_records(doc.ents), 'sentences': span_records(doc.sents),
            'noun_chunks': span_records(doc.noun_chunks), 'vector': float_values(doc.vector)})
    result = {'versions': {'spacy': spacy.__version__, 'thinc': thinc.__version__},
              'model': entry.model + ' ' + entry.version, 'input_sha256': hashlib.sha256(raw).hexdigest(),
              'cases': cases}
    output_path.write_text(json.dumps(result, ensure_ascii=False) + '\n')
    print(f'Wrote {len(cases)} cases, {token_count} tokens')


if __name__ == '__main__':
    main()
