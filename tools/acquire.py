"""Explicit acquisition of checksum-pinned official models for reference tooling."""
import argparse
import hashlib
import subprocess
from model_catalog import Release, catalog, release


class Options(argparse.Namespace):
    model: str = 'en_core_web_md'
    all: bool = False


def acquire(entry: Release) -> None:
    path = entry.wheel
    path.parent.mkdir(parents=True, exist_ok=True)
    if not path.exists():
        temporary = path.with_suffix('.download')
        subprocess.run(['curl', '--fail', '--location', '--retry', '2', entry.url,
                        '--output', str(temporary)], check=True)
        with temporary.open('rb') as stream:
            if hashlib.file_digest(stream, 'sha256').hexdigest() != entry.wheel_sha256:
                temporary.unlink()
                raise ValueError('Official model checksum mismatch')
        temporary.rename(path)
    with path.open('rb') as stream:
        if hashlib.file_digest(stream, 'sha256').hexdigest() != entry.wheel_sha256:
            raise ValueError('Official model checksum mismatch')
    subprocess.run(['uv', '--cache-dir', 'target/uv-cache', 'pip', 'install', '--python',
                    '.venv/bin/python', '--no-deps', str(path)], check=True)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    group = parser.add_mutually_exclusive_group()
    group.add_argument('--model', default='en_core_web_md')
    group.add_argument('--all', action='store_true')
    options = parser.parse_args(namespace=Options())
    for entry in catalog() if options.all else [release(options.model)]:
        acquire(entry)


if __name__ == '__main__':
    main()
