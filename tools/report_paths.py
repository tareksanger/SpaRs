"""Keep saved diagnostics portable without exposing host filesystem locations."""
from pathlib import Path
import re

from json_types import JsonValue

# URLs are source provenance, not filesystem paths. Protect them before scanning.
_URL = re.compile(r'https?://[^\s\"<>]+|</3(?![\w/])')
_ABSOLUTE = re.compile(r'(?<![\w/])(?:file:///?|[A-Za-z]:[\\/]|\\\\|/)(?=[^\W]|[.~])[^\s\"\'<>),;]+')


def portable_text(text: str, root: Path) -> str:
    urls: list[str] = []

    def protect(match: re.Match[str]) -> str:
        urls.append(match[0])
        return f'{{URL_{len(urls) - 1}}}'

    result = _URL.sub(protect, text)
    project = str(root.resolve())
    result = result.replace(project + '/', '').replace(project, '.')
    result = _ABSOLUTE.sub('<external-path>', result)
    for index, url in enumerate(urls):
        result = result.replace(f'{{URL_{index}}}', url)
    return result


def portable_json(value: JsonValue, root: Path) -> JsonValue:
    if isinstance(value, str):
        return portable_text(value, root)
    if isinstance(value, list):
        return [portable_json(item, root) for item in value]
    if isinstance(value, dict):
        return {portable_text(key, root): portable_json(item, root) for key, item in value.items()}
    return value
