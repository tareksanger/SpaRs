"""Instrument official networks, including greedy transition actions."""
import json
from pathlib import Path
import numpy as np
import spacy
from thinc.api import NumpyOps

def stages(model,d):
    if not len(d):return []
    embed=model.get_ref('embed').predict([d])[0]
    encode=model.get_ref('encode');pad=encode.attrs['pad']
    x=encode.ops.flatten([embed],pad=pad)
    out=[embed.tolist()]
    for layer in encode.layers[0].layers:
        x=layer.predict(x);out.append(x[pad:pad+len(d)].tolist())
    return out

def trace(pipe,d):
    if not len(d):return []
    step=pipe.model.predict([d]);state=pipe.moves.init_batch([d])[0];out=[]
    while not state.is_final():
        scores=step.predict([state])[0]
        valid=[bool(pipe.moves.is_valid(state,pipe.moves.get_class_name(i))) for i in range(pipe.moves.n_moves)]
        choice=max((i for i,v in enumerate(valid) if v),key=lambda i:scores[i])
        out.append({'ids':step.get_token_ids([state])[0].tolist(),'valid':valid,'scores':scores.tolist(),'action':choice})
        pipe.moves.apply_transition(state,pipe.moves.get_class_name(choice))
    return out

def main():
    n=spacy.load('en_core_web_md');result=[]
    for text in json.loads(Path('fixtures/development.json').read_text()):
        d=n.make_doc(text)
        record={'text':text,'tok2vec_stages':stages(n.get_pipe('tok2vec').model,d)}
        n.get_pipe('tok2vec')(d);n.get_pipe('tagger')(d)
        record['parser']=trace(n.get_pipe('parser'),d)
        n.get_pipe('parser')(d);n.get_pipe('attribute_ruler')(d);n.get_pipe('lemmatizer')(d)
        record['ner_stages']=stages(n.get_pipe('ner').model.get_ref('tok2vec').layers[0],d)
        record['ner']=trace(n.get_pipe('ner'),d)
        result.append(record)
    Path('fixtures/stages.expected.json').write_text(json.dumps(result))
    ops=NumpyOps();ids=np.array([0,1,2,2**64-1,1234567890123],dtype='uint64')
    Path('fixtures/hash.expected.json').write_text(json.dumps([{'id':int(i),'seed':s,'keys':row.tolist()} for s in range(8,14) for i,row in zip(ids,ops.hash(ids,s))]))
if __name__=='__main__':main()
