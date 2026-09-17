"""Check frozen fixtures, Markdown structure, links, and reviewer configuration."""
import hashlib
import json
import re
import tomllib
from pathlib import Path


def check_fixtures(root):
    lock = json.loads((root/'fixtures/checksums.json').read_text())
    actual = {str(p.relative_to(root)) for p in (root/'fixtures').glob('*.json') if p.name != 'checksums.json'}
    if actual != set(lock):
        raise ValueError('Fixture inventory changed. Review new fixtures and their checksums.')
    for name, digest in lock.items():
        if hashlib.sha256((root/name).read_bytes()).hexdigest() != digest:
            raise ValueError(f'Frozen fixture changed: {name}')


def check_markdown(path, root):
    lines = path.read_text().splitlines()
    fence = None
    paragraph = False
    for number, line in enumerate(lines, 1):
        stripped = line.strip()
        if stripped.startswith(('```', '~~~')):
            marker = stripped[:3]
            fence = None if fence == marker else marker
            paragraph = False
            continue
        if fence:
            continue
        structural = not stripped or stripped.startswith(('#', '|', '>', '<!--')) or re.match(r'^\s*(?:[-+*]|\d+[.)])\s', line)
        if not structural and paragraph:
            raise ValueError(f'{path}:{number}: keep a prose paragraph on one source line')
        paragraph = bool(stripped) and not structural
        for target in re.findall(r'\[[^\]]*\]\(([^)]+)\)', line):
            if '://' in target or target.startswith(('#', 'mailto:')):
                continue
            target = target.split('#')[0]
            if target.startswith('/') or not (path.parent/target).exists():
                raise ValueError(f'{path}:{number}: broken or nonportable link {target}')
    if fence:
        raise ValueError(f'{path}: unclosed code fence')


def check_agents(root):
    paths = sorted((root/'.codex/agents').glob('*.toml'))
    if len(paths) != 3:
        raise ValueError('Expected the three documented reviewer configurations.')
    names = set()
    for path in paths:
        data = tomllib.loads(path.read_text())
        for key in ('name', 'description', 'developer_instructions'):
            if not isinstance(data.get(key), str) or not data[key].strip():
                raise ValueError(f'{path}: missing {key}')
        if data['name'] != path.stem or data['name'] in names:
            raise ValueError(f'{path}: agent names must be unique and match filenames')
        if data.get('sandbox_mode') != 'read-only':
            raise ValueError(f'{path}: reviewers should be read-only')
        names.add(data['name'])


def main():
    root = Path(__file__).resolve().parent.parent
    check_fixtures(root)
    paths = [*root.glob('*.md'), *sorted((root/'docs').glob('*.md')), root/'fixtures/README.md', *sorted((root/'.github').glob('*.md'))]
    for path in paths:
        check_markdown(path, root)
    check_agents(root)
    print(f'PASS frozen fixture checksums, {len(paths)} Markdown files, and 3 reviewer configurations')


if __name__ == '__main__':
    main()
