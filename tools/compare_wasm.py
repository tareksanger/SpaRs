"""Compare secondary outputs to official fixtures; never redefine primary truth."""
import json
from pathlib import Path
reference=json.loads(Path('fixtures/development.expected.json').read_text())
secondary=json.loads(Path('reports/wasm-development.json').read_text())
assert len(reference['cases'])==len(secondary)
differences=[]
fields={'tag':'tag','pos':'pos','morph':'morphology','head':'head','dep':'dep','lemma':'lemma','idx':'idx','ws':'whitespace','is_sent_start':'sentence_start','ent_iob':'entity_iob','ent_type':'entity_type'}
for i,(c,w) in enumerate(zip(reference['cases'],secondary)):
 assert c['text']==w['text']
 out=w['output'];actual=[{v:t[k] for k,v in fields.items()} for t in out['tokens']]
 expected=[{v:t[v] for v in fields.values()} for t in c['tokens']]
 comparisons=[('tokens',actual,expected)]
 for dest,source in [('sentences','sents'),('noun_chunks','noun_chunks')]:
  actual=[(s['start_token'],s['end_token']) for s in out[source]]
  expected=[(s['start'],s['end']) for s in c[dest]]
  comparisons.append((dest,actual,expected))
 actual=[(s['start'],s['end'],s['label']) for s in out['ents']]
 expected=[(c['tokens'][s['start']]['idx'],len(c['text'].encode()[:c['tokens'][s['end']-1]['end']].decode()),s['label']) for s in c['entities']]
 comparisons.append(('entities_codepoint_offsets',actual,expected))
 for field,actual,expected in comparisons:
  if actual!=expected:differences.append({'case':i,'text':c['text'],'field':field,'secondary_actual':actual,'official_expected':expected})
Path('reports/wasm-mismatches.json').write_text(json.dumps({'cases':len(secondary),'official_versions':reference['versions'],'secondary_html_sha256':'8a60c2df6e88676970143455b72641c07d3a7e30b106517c3d4df178d8fa70c7','differences':differences},indent=2,ensure_ascii=False))
print(len(differences),'secondary field mismatches; native acceptance remains official Python parity')
