"""Export official installed model resources."""
import hashlib
import importlib.metadata as md
import json
from pathlib import Path
import argparse
import numpy as np
import spacy
import lexical
import provenance
import thinc
from safetensors.numpy import save_file
from spacy.symbols import IDS
from spacy.lang.norm_exceptions import BASE_NORMS
from spacy.strings import hash_string


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def export(out):
    assert spacy.__version__ == '3.8.14' and thinc.__version__ == '8.3.13'
    nlp = spacy.load('en_core_web_md')
    assert nlp.meta['version'] == '3.8.0'
    source_info=provenance.record()
    assert digest('assets/en_core_web_md-3.8.0-py3-none-any.whl')=='5e6329fe3fecedb1d1a02c3ea2172ee0fede6cea6e4aefb6a02d832dba78a310'
    assert nlp.pipe_names==['tok2vec','tagger','parser','attribute_ruler','lemmatizer','ner']
    out.mkdir(parents=True, exist_ok=True)
    tensors = {}
    def params(model, prefix):
        result = {}
        for name in model.param_names:
            key = f'{prefix}.{name}'
            tensors[key] = np.ascontiguousarray(model.get_param(name), dtype='float32')
            result[name] = key
        return result
    def tok2vec(model, prefix):
        embed, encode = model.get_ref('embed'), model.get_ref('encode')
        nodes = list(embed.walk())
        hashs = [x for x in nodes if x.name == 'hashembed']
        maxes = [x for x in nodes if x.name == 'maxout']
        norms = [x for x in nodes if x.name == 'layernorm']
        attrs = next(x.attrs['columns'] for x in nodes if x.name == 'extract_features')
        def block(m, norm, key):
            return {'maxout': params(m,key+'.max'), 'norm': params(norm,key+'.norm')}
        enc = list(encode.walk())
        return {'width':model.get_dim('nO'), 'attrs':attrs,
            'hashes':[{'seed':x.attrs['seed'], 'params':params(x,f'{prefix}.hash{i}')} for i,x in enumerate(hashs)],
            'static':params(next(x for x in nodes if x.name=='static_vectors'),prefix+'.static'),
            'mix':block(maxes[0],norms[0],prefix+'.mix'),
            'pad':encode.attrs['pad'],
            'windows':[x.attrs['window_size'] for x in enc if x.name=='expand_window'],
            'layers':[block(m,l,f'{prefix}.enc{i}') for i,(m,l) in enumerate(zip([x for x in enc if x.name=='maxout'],[x for x in enc if x.name=='layernorm']))]}
    shared = tok2vec(nlp.get_pipe('tok2vec').model,'tok2vec')
    tagger = nlp.get_pipe('tagger')
    tag = {'labels':list(tagger.labels),'params':params(tagger.model.layers[1].layers[0],'tagger')}
    transitions = {}
    for name in ('parser','ner'):
        pipe = nlp.get_pipe(name); m = pipe.model
        assert pipe.cfg['beam_width']==1 and not m.attrs['unseen_classes'] and m.attrs['has_upper']
        transitions[name] = {'actions':[pipe.moves.get_class_name(i) for i in range(pipe.moves.n_moves)],
            'reduce':params(m.get_ref('tok2vec').layers[-1],name+'.reduce'),
            'lower':params(m.get_ref('lower'),name+'.lower'), 'upper':params(m.get_ref('upper'),name+'.upper')}
        if name == 'ner': transitions[name]['tok2vec'] = tok2vec(m.get_ref('tok2vec').layers[0],'ner.tok2vec')
    tokenizer = nlp.tokenizer
    def pattern(fn): return fn.__self__.pattern if fn else None
    rules = {k:[{nlp.vocab.strings[a]:v for a,v in t.items()} for t in v] for k,v in tokenizer.rules.items()}
    tables = nlp.get_pipe('lemmatizer').lookups
    # Table outer keys are hashed POS identifiers; recover explicitly.
    lemmas = {name:{pos:tables.get_table(name).get(pos,{}) for pos in ('noun','verb','adj','adv','punct')} for name in tables.tables}
    tensors['vectors'] = nlp.vocab.vectors.data
    save_file(tensors,str(out/'weights.safetensors'))
    lexical_data=lexical.export(nlp)
    original_patterns={k:pattern(v) for k,v in [('prefix',tokenizer.prefix_search),('suffix',tokenizer.suffix_search),('infix',tokenizer.infix_finditer),('url',tokenizer.url_match),('token_match',tokenizer.token_match)]}
    native_patterns={k:lexical.translate_pattern(v,lexical_data) for k,v in original_patterns.items()}
    manifest = {'format_version':1,'model':'en_core_web_md','model_version':'3.8.0',
        'versions':{p:md.version(p) for p in ('spacy','thinc','numpy','murmurhash','srsly')},
        'pipeline':nlp.pipe_names,'config':nlp.config.to_str(),'metadata':nlp.meta,
        'source':'https://github.com/explosion/spacy-models/releases/tag/en_core_web_md-3.8.0',
        'wheel_sha256':digest('assets/en_core_web_md-3.8.0-py3-none-any.whl'),
        'weights_sha256':digest(out/'weights.safetensors'),
        'tensors':{k:{'shape':list(v.shape),'dtype':'F32'} for k,v in tensors.items()},
        'tokenizer':{**native_patterns,'python_patterns':original_patterns,'regex_dialect':'fancy-regex-explicit-python-unicode-v1','rules':rules},
        'norms':{**{str(hash_string(k)):v for k,v in BASE_NORMS.items()}, **{str(k):v for k,v in nlp.vocab.lookups.get_table('lexeme_norm').items()}},
        'lexical':lexical_data,'symbols':IDS,'tok2vec':shared,'tagger':tag,**transitions,
        'attribute_rules':nlp.get_pipe('attribute_ruler').patterns,'lemmas':lemmas,
        'vector_keys':{str(k):v for k,v in nlp.vocab.vectors.key2row.items()}}
    (out/'manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,separators=(',',':'),sort_keys=True))
    (out/'source-lock.json').write_text(json.dumps(source_info,indent=2,sort_keys=True))
    for notice in ('spacy-MIT.txt','thinc-MIT.txt','Python.txt','Unicode.txt'):
        (out/notice).write_bytes((Path('licenses')/notice).read_bytes())
    modeldir = Path(md.distribution('en_core_web_md').locate_file('en_core_web_md/en_core_web_md-3.8.0'))
    for name in ('LICENSE','LICENSES_SOURCES'):
        (out/name).write_bytes((modeldir/name).read_bytes())
    print(f'exported {len(tensors)} tensors to {out}')

if __name__ == '__main__':
    p=argparse.ArgumentParser();p.add_argument('--out',type=Path,default=Path('assets/en_core_web_md-3.8.0'));export(p.parse_args().out)
