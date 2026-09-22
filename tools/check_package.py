"""Verify the release archive and run a native consumer against its library."""
import json
import os
from pathlib import Path, PurePosixPath
import shutil
import subprocess
import tarfile
import tempfile
import tomllib

from json_types import json_object, json_string, validate_json


def validate_inventory(files: set[str], sources: set[str]) -> None:
    required = sources | {
        'Cargo.toml', 'Cargo.toml.orig', 'Cargo.lock', 'README.md', 'LICENSE',
        'THIRD_PARTY_NOTICES.md', 'licenses/spacy-MIT.txt', 'licenses/thinc-MIT.txt',
    }
    missing = required - files
    unexpected = files - required - {'.cargo_vcs_info.json'}
    if missing or unexpected:
        raise ValueError(f'Invalid release inventory: missing={sorted(missing)}, unexpected={sorted(unexpected)}')


def main() -> None:
    root = Path(__file__).resolve().parent.parent
    manifest = json_object(validate_json(tomllib.loads((root / 'Cargo.toml').read_text())))
    package = json_object(manifest['package'])
    name, version = json_string(package['name']), json_string(package['version'])
    prefix = f'{name}-{version}'
    subprocess.run(['cargo', 'package', '--allow-dirty', '--offline', '--target-dir', 'target'], cwd=root, check=True)
    archive = root / 'target/package' / f'{prefix}.crate'
    with tarfile.open(archive, 'r:gz') as contents:
        files = {str(PurePosixPath(member.name).relative_to(prefix)) for member in contents if member.isfile()}
    sources = {path.relative_to(root).as_posix() for path in (root / 'src').rglob('*.rs') if path.name != 'tests.rs'}
    validate_inventory(files, sources)
    packaged = root / 'target/package' / prefix
    build = root / 'target/package-consumer-build'
    (root / 'target').mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='package-consumer-', dir=root / 'target') as temporary:
        consumer = Path(temporary)
        (consumer / 'src').mkdir()
        shutil.copyfile(root / 'consumer/src/main.rs', consumer / 'src/main.rs')
        dependency = json.dumps(os.path.relpath(packaged, consumer))
        (consumer / 'Cargo.toml').write_text(
            '[package]\nname = "packaged-consumer-check"\nversion = "0.0.0"\n'
            'edition = "2021"\npublish = false\n\n[dependencies]\n'
            f'{name} = {{ path = {dependency} }}\n\n[workspace]\n')
        subprocess.run([
            'cargo', 'build', '--release', '--offline', '--manifest-path',
            str((consumer / 'Cargo.toml').relative_to(root)),
            '--target-dir', str(build.relative_to(root)),
        ], cwd=root, check=True)
        subprocess.run([
            str(build.relative_to(root) / 'release/packaged-consumer-check'),
            'assets/en_core_web_md-3.8.0',
        ], cwd=root, env={'PATH': ''}, check=True)
    print(f'PASS release inventory ({len(files)} files) and separate packaged consumer')


if __name__ == '__main__':
    main()
