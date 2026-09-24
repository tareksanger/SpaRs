"""Typed, checksum-pinned official releases shared with the native installer."""
from dataclasses import dataclass
from pathlib import Path
from json_types import json_array, json_int, json_object, json_string, read_json


@dataclass(frozen=True)
class Limits:
    archive: int
    entry: int
    expanded: int
    installed_file: int


@dataclass(frozen=True)
class Release:
    model: str
    version: str
    wheel_sha256: str
    url: str
    format_version: int
    limits: Limits

    @property
    def wheel(self) -> Path:
        return Path('assets') / f'{self.model}-{self.version}-py3-none-any.whl'

    @property
    def prefix(self) -> str:
        return f'{self.model}/{self.model}-{self.version}/'


def catalog() -> list[Release]:
    result: list[Release] = []
    for raw in json_array(read_json(Path(__file__).resolve().parent.parent / 'models/catalog.json')):
        value = json_object(raw)
        limits = json_object(value['limits'])
        result.append(Release(json_string(value['model']), json_string(value['version']),
            json_string(value['wheel_sha256']), json_string(value['url']), json_int(value['format_version']),
            Limits(*(json_int(limits[key]) for key in ('archive', 'entry', 'expanded', 'installed_file')))))
    return result


def release(model: str, version: str = '3.8.0') -> Release:
    for entry in catalog():
        if (entry.model, entry.version) == (model, version):
            return entry
    raise ValueError(f'No pinned acquisition record for {model} {version}')
