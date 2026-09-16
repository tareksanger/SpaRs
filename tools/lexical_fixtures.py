import json
from pathlib import Path
import spacy
n=spacy.load('en_core_web_md')
manifest=json.loads(Path('assets/en_core_web_md-3.8.0/manifest.json').read_text())
words={'hello','Apple','DOG','dog','Cats','ǅabc','123','²','Ⅷ','٣rd','one','twenty-first','3/4','+1,234.00','¿!?','€','www.a.org','foo@example.com','word.foo','http://anything','https://user@example.com','U.S.A.','  ','\t','ΣΟΣ','İ'}
# Boundary tests for every exported Unicode classification range.
for ranges in manifest['lexical']['ranges'].values():
 for a,b in ranges:
  for i in (a-1,a,b,b+1):
   if 0<=i<=0x10ffff and not 0xd800<=i<=0xdfff:words.add(chr(i))
fields=['is_alpha','is_digit','is_lower','is_upper','is_title','is_space','is_ascii','is_punct','is_currency','is_stop','is_bracket','is_quote','is_left_punct','is_right_punct','like_num','like_email','like_url','has_vector']
rows=[]
for s in sorted(words):
 l=n.vocab[s]
 rows.append({'text':s,'expected':{'orth':l.orth,'norm':l.norm_,'shape':l.shape_,'prefix':l.prefix_,'suffix':l.suffix_,**{k:getattr(l,k) for k in fields}}})
Path('fixtures/lexical.expected.json').write_text(json.dumps(rows,ensure_ascii=False))
vecwords=['dog','cat','Apple','Microsoft','Zzxyqnotaword123','the','hello',' ']
import warnings
with warnings.catch_warnings():
 warnings.simplefilter('ignore')
 vectors={'words':[{'text':s,'vector':n.vocab[s].vector.tolist(),'has_vector':n.vocab[s].has_vector} for s in vecwords], 'similarities':[{'a':a,'b':b,'expected':n.make_doc(a).similarity(n.make_doc(b))} for a,b in [('dog','cat'),('',''),('','dog'),('xyznope','xyznope'),('xyznope','zyxother'),('Apple and Microsoft','Microsoft and Apple')]]}
Path('fixtures/vectors.expected.json').write_text(json.dumps(vectors))
print(len(rows),'lexical cases')
