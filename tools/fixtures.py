"""Frozen input corpora; official reference outputs and intermediate activations."""
import json
from pathlib import Path
import spacy,numpy as np,thinc
from spacy.attrs import NORM,PREFIX,SUFFIX,SHAPE,SPACY,IS_SPACE

from typing import TypedDict
from reference_types import TokenRecord, SpanRecord, token_records, span_records, float_rows, float_values, uint_rows, predict_docs
from json_types import read_json, json_array, json_string

class FixtureCase(TypedDict):
    text: str
    features: list[list[int]]
    tok2vec: list[list[float]]
    tokens: list[TokenRecord]
    entities: list[SpanRecord]
    sentences: list[SpanRecord]
    noun_chunks: list[SpanRecord]
    vector: list[float]


def run(split: str) -> None:
    n=spacy.load('en_core_web_md');results: list[FixtureCase] = []
    for text in [json_string(item) for item in json_array(read_json(Path(f'fixtures/{split}.json')))]:
        d=n.make_doc(text);x=predict_docs(n.get_pipe('tok2vec').model, [d])[0] if len(d) else np.zeros((0,96),dtype='f')
        features=uint_rows(d.to_array([NORM,PREFIX,SUFFIX,SHAPE,SPACY,IS_SPACE]))
        d=n(d)
        results.append({'text':text,'features':features,'tok2vec':float_rows(x),
            'tokens':token_records(d,text),
            'entities':span_records(d.ents),'sentences':span_records(d.sents),'noun_chunks':span_records(d.noun_chunks),'vector':float_values(d.vector)})
    Path(f'fixtures/{split}.expected.json').write_text(json.dumps({'versions':{'spacy':spacy.__version__,'thinc':thinc.__version__},'cases':results},ensure_ascii=False))
if __name__=='__main__':
    import sys
    run(sys.argv[1] if len(sys.argv)>1 else 'development')
