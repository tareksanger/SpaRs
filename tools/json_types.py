"""Validate untrusted JSON before passing concrete records to tooling."""
from dataclasses import dataclass
import json
from pathlib import Path
from typing import TypeGuard
from collections.abc import Mapping, Sequence

type JsonValue = None | bool | int | float | str | list[JsonValue] | dict[str, JsonValue]


def _mapping(value: object) -> TypeGuard[Mapping[object, object]]:
    return isinstance(value, dict)


def _sequence(value: object) -> TypeGuard[Sequence[object]]:
    return isinstance(value, list)


def validate_json(value: object) -> JsonValue:
    if value is None or isinstance(value, (bool, int, float, str)):
        return value
    if _sequence(value):
        return [validate_json(item) for item in value]
    if _mapping(value):
        result: dict[str, JsonValue] = {}
        for key, item in value.items():
            if not isinstance(key, str):
                raise ValueError('JSON object keys must be strings')
            result[key] = validate_json(item)
        return result
    raise ValueError(f'Unsupported JSON value: {type(value).__name__}')


def parse_json(text: str) -> JsonValue:
    value: object = json.loads(text)
    return validate_json(value)


def read_json(path: Path) -> JsonValue:
    return parse_json(path.read_text())


def json_object(value: JsonValue) -> dict[str, JsonValue]:
    if not isinstance(value, dict):
        raise ValueError('Expected a JSON object')
    return value


def json_array(value: JsonValue) -> list[JsonValue]:
    if not isinstance(value, list):
        raise ValueError('Expected a JSON array')
    return value


def json_string(value: JsonValue) -> str:
    if not isinstance(value, str):
        raise ValueError('Expected a JSON string')
    return value


def json_int(value: JsonValue) -> int:
    if isinstance(value, bool) or not isinstance(value, int):
        raise ValueError('Expected a JSON integer')
    return value


def json_float(value: JsonValue) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise ValueError('Expected a JSON number')
    return float(value)


def string_map(value: JsonValue) -> dict[str, str]:
    return {key: json_string(item) for key, item in json_object(value).items()}


@dataclass(frozen=True)
class ModelMetadata:
    model: str
    model_version: str
    versions: dict[str, str]
    weights_sha256: str

    @classmethod
    def load(cls, path: Path) -> 'ModelMetadata':
        value = json_object(read_json(path))
        return cls(json_string(value['model']), json_string(value['model_version']),
                   string_map(value['versions']), json_string(value['weights_sha256']))
