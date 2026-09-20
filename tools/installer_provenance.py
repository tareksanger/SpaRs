"""Verify installed sources while preserving the pinned recipe's provenance bytes."""
from pathlib import Path
from json_types import JsonValue, json_object, json_string, read_json, string_map
from provenance import PackageRecord


def source_records(value: JsonValue) -> dict[str, PackageRecord]:
    records: dict[str, PackageRecord] = {}
    for name, raw in json_object(value).items():
        record = json_object(raw)
        if set(record) != {'version', 'release', 'files'}:
            raise ValueError('Source records require version, release and files')
        files = string_map(record['files'])
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
    if source_records(observed) != source_records(read_json(path)):
        raise ValueError('Installed source files differ from the pinned reference')
    # Keep the existing immutable installation identity and every recorded notice.
    return path.read_bytes()
