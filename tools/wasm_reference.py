"""Optional secondary reference extraction. Requires separately downloaded HTML."""
import base64,gzip,hashlib,json,re
from pathlib import Path
p=Path('reference/wasm-demo.html');data=p.read_bytes();sha=hashlib.sha256(data).hexdigest()
expected='8a60c2df6e88676970143455b72641c07d3a7e30b106517c3d4df178d8fa70c7'
if sha!=expected:raise RuntimeError(f'Mutable reference changed: expected {expected}, got {sha}; inspect before use')
s=data.decode();out=Path('reference/wasm');out.mkdir(exist_ok=True)
for key in ('WASM','MAN','ST','K2R','R2W'):
    match=re.search(r'\b'+key+r'\s*=\s*[\'\"]([^\'\"]+)[\'\"]',s)
    if not match:raise RuntimeError('missing asset '+key)
    (out/key).write_bytes(gzip.decompress(base64.b64decode(match.group(1))))
# Select only the wasm_bindgen generated closure, never execute the page UI.
start=s.index('let wasm_bindgen');end=s.index('</script>',start)
(out/'bindings.js').write_text(s[start:end]+'\nmodule.exports = wasm_bindgen;\n')
(out/'provenance.json').write_text(json.dumps({'url':'https://raw.githubusercontent.com/maymay-wa/spacy-wasm/refs/heads/main/spacy-rt-demo.html','html_sha256':sha,'assets':{k:{'size':(out/k).stat().st_size,'sha256':hashlib.sha256((out/k).read_bytes()).hexdigest()} for k in ('WASM','MAN','ST','K2R','R2W')}},indent=2))
