"""Verify installed sources while preserving the pinned recipe's provenance bytes."""
from pathlib import Path
from json_types import JsonValue, json_object, json_string, read_json, string_map
from provenance import PackageRecord

# Official CPython 3.12 wheels differ only in build-directory comments in this
# generated file. Both complete file hashes were verified; see reference/README.md.
_LEVENSHTEIN_SOURCE = 'spacy/matcher/levenshtein.c'
_MACOS_SHA256 = '3d0aaaea19850900ec4071700ee6fe69c7d8e59ee1a4f9c963189e78161a278d'
_LINUX_SHA256 = 'e14722922674055c3d72e21fd8b727597c58ba7395f70be68eaa55aa15bd44a9'


def source_records(value: JsonValue) -> dict[str, PackageRecord]:
    records: dict[str, PackageRecord] = {}
    for name, raw in json_object(value).items():
        record = json_object(raw)
        if set(record) != {'version', 'release', 'files'}:
            raise ValueError('Source records require version, release and files')
        files = string_map(record['files'])
        if name == 'spacy' and record['version'] == '3.8.14' and files.get(_LEVENSHTEIN_SOURCE) == _LINUX_SHA256:
            files[_LEVENSHTEIN_SOURCE] = _MACOS_SHA256
        # These four files describe the local Python installation, not model assets.
        if name == 'en_core_web_md':
            prefix = 'en_core_web_md-3.8.0.dist-info/'
            for local in ('INSTALLER', 'REQUESTED', 'direct_url.json', 'uv_cache.json'):
                files.pop(prefix + local, None)
        records[name] = {'version': json_string(record['version']),
                         'release': json_string(record['release']), 'files': files}
    return records


def pinned_source_lock(observed: JsonValue) -> bytes:
    path = Path('reference/source-lock.json')
    actual = source_records(observed)
    expected = source_records(read_json(path))
    if actual != expected:
        changed = sorted(name for name in actual.keys() | expected.keys() if actual.get(name) != expected.get(name))
        raise ValueError('Installed source files differ from the pinned reference: ' + ', '.join(changed))
    # Keep the existing immutable installation identity and every recorded notice.
    return path.read_bytes()


def model_source_lock(observed: JsonValue, model_name: str) -> bytes:
    """Pin shared implementation sources and omit machine-specific installation records."""
    import json
    import hashlib
    import zipfile
    from model_catalog import release
    entry = release(model_name)
    actual = source_records(observed)
    expected = source_records(read_json(Path('reference/source-lock.json')))
    for name in ('spacy', 'thinc', 'murmurhash'):
        if actual.get(name) != expected[name]:
            raise ValueError('Installed source files differ from the pinned reference: ' + name)
    if set(actual) != {'spacy', 'thinc', 'murmurhash', model_name}:
        raise ValueError('Unexpected model source inventory')
    model = actual[model_name]
    prefix = f"{model_name}-{model['version']}.dist-info/"
    for local in ('INSTALLER', 'REQUESTED', 'direct_url.json', 'uv_cache.json'):
        model['files'].pop(prefix + local, None)
    with entry.wheel.open('rb') as archive:
        if hashlib.file_digest(archive, 'sha256').hexdigest() != entry.wheel_sha256:
            raise ValueError('Official wheel checksum mismatch')
    with zipfile.ZipFile(entry.wheel) as wheel:
        files = {name: hashlib.sha256(wheel.read(name)).hexdigest() for name in wheel.namelist()
                 if not name.endswith('/') and not name.endswith('.dist-info/RECORD')}
    expected_model: PackageRecord = {'version': entry.version,
        'release': f'https://pypi.org/project/{entry.model}/{entry.version}/', 'files': files}
    if model != expected_model:
        raise ValueError('Installed model files differ from the pinned official wheel: ' + model_name)
    return (json.dumps(actual, indent=2, sort_keys=True) + '\n').encode()
