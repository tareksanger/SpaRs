"""All official exceptions, composed affixes, and Unicode/whitespace probes."""
import json
from pathlib import Path
import spacy
from spacy.attrs import NORM,PREFIX,SUFFIX,SHAPE,SPACY,IS_SPACE
n=spacy.load('en_core_web_md')
inputs=sorted(set(n.tokenizer.rules)|{f'({s})' for s in n.tokenizer.rules}|{f'“{s}!”' for s in n.tokenizer.rules})
inputs += ['a'+s+'b' for s in [' ','  ','\t','\n','\r\n','\u001c','\u00a0','\u2003']]
inputs += ['www.example.org','https://user@example.org:80/path?q=x#y','a.b@example.net','3.14','½ ² Ⅷ ١٢३ ǅ ᾈ अि ΣΟΣ','㌀ 𐐀 𐐨','a\u200db','\n\n',' ','   ','e.g., viz. i.e.','abcdefghijklmnopqrstuvwxyz'*5]
rows=[]
for text in inputs:
 d=n.make_doc(text)
 rows.append({'text':text,'tokens':[{'text':t.text,'idx':t.idx,'whitespace':bool(t.whitespace_),'norm':t.norm_} for t in d], 'features':d.to_array([NORM,PREFIX,SUFFIX,SHAPE,SPACY,IS_SPACE]).tolist()})
Path('fixtures/tokenizer.expected.json').write_text(json.dumps(rows,ensure_ascii=False))
print(len(rows),'tokenizer cases')
