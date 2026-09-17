"""Check source files against wheel RECORD entries; pin the files actually read."""
import base64
import hashlib
import importlib.metadata as md
import json
from pathlib import Path

from typing import TypedDict

class PackageRecord(TypedDict):
    version: str
    release: str
    files: dict[str, str]


def record() -> dict[str, PackageRecord]:
    packages: dict[str, PackageRecord] = {}
    for name in ('spacy','thinc','murmurhash','en_core_web_md'):
        d=md.distribution(name);files: dict[str, str] = {}
        for f in d.files or []:
            if not (name=='en_core_web_md' and f.hash) and f.suffix not in ('.py','.pyx','.pxd','.pxi','.h','.cpp','.c') and 'LICENSE' not in str(f):continue
            path=Path(str(d.locate_file(f)))
            sha=hashlib.sha256(path.read_bytes()).digest()
            if f.hash and f.hash.mode=='sha256':
                assert base64.urlsafe_b64encode(sha).decode().rstrip('=')==f.hash.value, f'wheel RECORD mismatch: {f}'
            files[str(f)]=sha.hex()
        packages[name]={'version':d.version,'release':f'https://pypi.org/project/{name}/{d.version}/','files':files}
    return packages
if __name__=='__main__':
    Path('reference/source-lock.json').write_text(json.dumps(record(),indent=2,sort_keys=True))
