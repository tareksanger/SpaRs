"""Type-check and execute TypeScript guide examples against the native addon."""
import subprocess
import re
import tempfile
from pathlib import Path


def examples(text: str) -> list[str]:
    blocks: list[str] = []
    fence: str | None = None
    current: list[str] | None = None
    for line in text.splitlines():
        match = re.match(r'^ {0,3}(`{3,}|~{3,})(.*)$', line)
        if fence is not None:
            if match and match[1][0] == fence[0] and len(match[1]) >= len(fence) and not match[2].strip():
                if current is not None:
                    blocks.append('\n'.join(current) + '\n')
                fence = None
                current = None
            elif current is not None:
                current.append(line)
        elif match:
            fence = match[1]
            current = [] if line == '```typescript' else None
    if current is not None:
        raise ValueError('Unclosed TypeScript example.')
    return blocks


def run_example(root: Path, source: str) -> subprocess.CompletedProcess[str]:
    # The temporary source sits beside index.js so guide imports work unchanged.
    with tempfile.NamedTemporaryFile(mode='w', suffix='.mts', prefix='.spars-doc-',
                                     dir=root/'bindings/node', delete=False) as handle:
        handle.write(source)
        path = Path(handle.name)
    try:
        checked = subprocess.run([
            str(root/'bindings/node/node_modules/.bin/tsc'), '--noEmit', '--strict',
            '--noUncheckedIndexedAccess', '--exactOptionalPropertyTypes',
            '--typeRoots', str(root/'bindings/node/node_modules/@types'),
            '--module', 'nodenext', '--target', 'es2023', str(path),
        ], cwd=root, text=True, capture_output=True)
        if checked.returncode:
            return checked
        return subprocess.run(['node', str(path)], cwd=root, text=True, capture_output=True)
    finally:
        path.unlink()
