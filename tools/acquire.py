"""Explicit network boundary for the pinned official model (Python stdlib only)."""
import hashlib
from pathlib import Path
import subprocess

URL='https://github.com/explosion/spacy-models/releases/download/en_core_web_md-3.8.0/en_core_web_md-3.8.0-py3-none-any.whl'
SHA256='5e6329fe3fecedb1d1a02c3ea2172ee0fede6cea6e4aefb6a02d832dba78a310'
p=Path('assets/en_core_web_md-3.8.0-py3-none-any.whl');p.parent.mkdir(exist_ok=True)
if not p.exists():
    temp=p.with_suffix('.download')
    subprocess.run(['curl','--fail','--location','--retry','2',URL,'--output',str(temp)],check=True)
    if hashlib.sha256(temp.read_bytes()).hexdigest()!=SHA256:raise RuntimeError('official model checksum mismatch')
    temp.rename(p)
if hashlib.sha256(p.read_bytes()).hexdigest()!=SHA256:raise RuntimeError('official model checksum mismatch')
subprocess.run(['uv','pip','install','--python','.venv/bin/python','--no-deps',str(p)],check=True)
