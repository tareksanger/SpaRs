"""Direct regex span agreement; independent of tokenizer control flow."""
import json
from pathlib import Path
import spacy
n=spacy.load('en_core_web_md');t=n.tokenizer
chars={row['text'] for row in json.loads(Path('fixtures/lexical.expected.json').read_text()) if len(row['text'])==1}
rows=[]
for c in sorted(chars):
 for text in (c+c+'://example.org','https://example.org/'+c,'(a'+c+'b.)'):
  single=lambda f: ([[m.start(),m.end()]] if (m:=f(text)) else [])
  rows.append({'text':text,'prefix':single(t.prefix_search),'suffix':single(t.suffix_search),'infix':[[m.start(),m.end()] for m in t.infix_finditer(text)],'url':single(t.url_match)})
Path('fixtures/regex.expected.json').write_text(json.dumps(rows,ensure_ascii=False))
print(len(rows),'regex cases')
