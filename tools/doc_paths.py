"""Reject machine-specific paths in Markdown, including executable examples."""
from pathlib import Path
import re

_URL_START = re.compile(r'https?://')
_CLOSING_TAG = re.compile(r'</[A-Za-z][A-Za-z0-9-]*\s*>')
_ABSOLUTE = re.compile(
    r'(?<![\w./\\-])(?:file:(?://)?[/\\]|[A-Za-z]:[/\\]|\\\\[^\\\s]+\\|//(?=[\w.-]+/)|~[/\\]|/)(?=[\w.~])')
_EXPANSION = re.compile(r'\$(?:HOME|PWD)\b|\$\{(?:HOME|PWD)\}|%(?:USERPROFILE|HOMEPATH)%')


def without_urls(line: str) -> str:
    result: list[str] = []
    position = 0
    while match := _URL_START.search(line, position):
        result.append(line[position:match.start()])
        end = match.end()
        parentheses = 0
        brackets = 0
        while end < len(line):
            character = line[end]
            if character.isspace() or character in '<>"\'`':
                break
            if character == '(':
                parentheses += 1
            elif character == ')':
                if parentheses == 0:
                    break
                parentheses -= 1
            elif character == '[':
                brackets += 1
            elif character == ']':
                if brackets == 0:
                    break
                brackets -= 1
            end += 1
        result.append(' ')
        position = end
    result.append(line[position:])
    return ''.join(result)


def check_paths(line: str, path: Path, root: Path, number: int) -> None:
    # Upstream URLs and HTML closing tags are not filesystem paths. The leading
    # boundary also preserves ./ and ../ paths, comments, and ordinary division.
    checked = _CLOSING_TAG.sub('', without_urls(line))
    if _ABSOLUTE.search(checked) or _EXPANSION.search(checked):
        label = path.relative_to(root)
        raise ValueError(f'{label}:{number}: use project-relative filesystem paths')
