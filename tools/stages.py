"""Instrument official networks, including greedy transition actions."""
import json
from pathlib import Path
import numpy as np
import spacy
from thinc.api import NumpyOps

from typing import TypedDict
from reference_types import Model, Doc, Parser, integer, float_rows, float_values, predict_docs, predict_array, predict_step
from json_types import read_json, json_array, json_string

class ActionRecord(TypedDict):
    ids: list[int]
    valid: list[bool]
    scores: list[float]
    action: int

class StageRecord(TypedDict):
    text: str
    tok2vec_stages: list[list[list[float]]]
    parser: list[ActionRecord]
    ner_stages: list[list[list[float]]]
    ner: list[ActionRecord]


def stages(model: Model, d: Doc) -> list[list[list[float]]]:
    if not len(d):return []
    embed=predict_docs(model.get_ref('embed'), [d])[0]
    encode=model.get_ref('encode');pad=integer(encode.attrs['pad'])
    x=encode.ops.flatten([embed],pad=pad)
    out=[float_rows(embed)]
    for layer in encode.layers[0].layers:
        x=predict_array(layer, x);out.append(float_rows(x[pad:pad+len(d)]))
    return out

def trace(pipe: Parser, d: Doc) -> list[ActionRecord]:
    if not len(d):return []
    step=predict_step(pipe.model, [d]);state=pipe.moves.init_batch([d])[0];out: list[ActionRecord] = []
    while not state.is_final():
        scores=step.predict([state])[0]
        valid=[bool(pipe.moves.is_valid(state,pipe.moves.get_class_name(i))) for i in range(pipe.moves.n_moves)]
        choice=max((i for i,v in enumerate(valid) if v),key=lambda i:scores[i])
        out.append({'ids':[int(value) for value in step.get_token_ids([state])[0]],'valid':valid,'scores':float_values(scores),'action':choice})
        pipe.moves.apply_transition(state,pipe.moves.get_class_name(choice))
    return out

def main() -> None:
    n=spacy.load('en_core_web_md');result: list[StageRecord] = []
    for text in [json_string(item) for item in json_array(read_json(Path('fixtures/development.json')))]:
        d=n.make_doc(text)
        tok2vec_stages=stages(n.get_pipe('tok2vec').model,d)
        n.get_pipe('tok2vec')(d);n.get_pipe('tagger')(d)
        parser_trace=trace(n.get_pipe('parser'),d)
        n.get_pipe('parser')(d);n.get_pipe('attribute_ruler')(d);n.get_pipe('lemmatizer')(d)
        ner_stages=stages(n.get_pipe('ner').model.get_ref('tok2vec').layers[0],d)
        ner_trace=trace(n.get_pipe('ner'),d)
        result.append({'text': text, 'tok2vec_stages': tok2vec_stages, 'parser': parser_trace, 'ner_stages': ner_stages, 'ner': ner_trace})
    Path('fixtures/stages.expected.json').write_text(json.dumps(result))
    ops=NumpyOps();ids=np.array([0,1,2,2**64-1,1234567890123],dtype='uint64')
    Path('fixtures/hash.expected.json').write_text(json.dumps([{'id':int(i),'seed':s,'keys':[int(value) for value in row]} for s in range(8,14) for i,row in zip(ids,ops.hash(ids,s))]))
if __name__=='__main__':main()
