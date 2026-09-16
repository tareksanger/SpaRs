"""Frozen input corpora; official reference outputs and intermediate activations."""
import json
from pathlib import Path
import spacy,numpy as np,thinc
from spacy.attrs import NORM,PREFIX,SUFFIX,SHAPE,SPACY,IS_SPACE

def run(split):
    n=spacy.load('en_core_web_md');results=[]
    for text in json.loads(Path(f'fixtures/{split}.json').read_text()):
        d=n.make_doc(text);x=n.get_pipe('tok2vec').model.predict([d])[0] if len(d) else np.zeros((0,96),dtype='f')
        features=d.to_array([NORM,PREFIX,SUFFIX,SHAPE,SPACY,IS_SPACE]).tolist()
        d=n(d)
        spans=lambda spans:[{'start':s.start,'end':s.end,'label':s.label_} for s in spans]
        results.append({'text':text,'features':features,'tok2vec':x.tolist(),
            'tokens':[{'start':len(text[:t.idx].encode()),'end':len(text[:t.idx+len(t)].encode()),'idx':t.idx,'whitespace':bool(t.whitespace_),'norm':t.norm_,'tag':t.tag_,'pos':t.pos_,'morphology':str(t.morph),'lemma':t.lemma_,'head':t.head.i,'dep':t.dep_,'sentence_start':t.is_sent_start,'entity_iob':t.ent_iob_,'entity_type':t.ent_type_} for t in d],
            'entities':spans(d.ents),'sentences':spans(d.sents),'noun_chunks':spans(d.noun_chunks),'vector':d.vector.tolist()})
    Path(f'fixtures/{split}.expected.json').write_text(json.dumps({'versions':{'spacy':spacy.__version__,'thinc':thinc.__version__},'cases':results},ensure_ascii=False))
if __name__=='__main__':
    import sys
    run(sys.argv[1] if len(sys.argv)>1 else 'development')
