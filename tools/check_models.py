"""Re-export catalog models and regenerate their separately frozen reference cases."""
from pathlib import Path
import hashlib
import json
from dataclasses import asdict, dataclass
import subprocess
import tempfile
from check_reference import Counts, compare
from export import export
from json_types import read_json
from model_catalog import catalog
from model_reference import generate


@dataclass(frozen=True)
class ModelReport:
    model: str
    fixture_sha256: str
    comparison: Counts


def main() -> None:
    Path('target').mkdir(exist_ok=True)
    reports: list[ModelReport] = []
    with tempfile.TemporaryDirectory(prefix='model-check-', dir='target') as temporary:
        root = Path(temporary)
        for entry in catalog():
            if entry.format_version == 1:
                continue  # Legacy model re-export and reference checks run in verify.py.
            output = root / entry.model
            export(output, entry.model)
            committed = Path('assets') / f'{entry.model}-{entry.version}'
            for name in ('manifest.json', 'weights.safetensors', 'source-lock.json', 'LICENSE', 'LICENSES_SOURCES'):
                with (output / name).open('rb') as actual, (committed / name).open('rb') as expected:
                    if hashlib.file_digest(actual, 'sha256').digest() != hashlib.file_digest(expected, 'sha256').digest():
                        raise ValueError(f'{entry.model} re-export differs: {name}')
            expected_fixture = Path('fixtures') / f'model-{entry.model.rsplit("_", 1)[1]}-v1.expected.json'
            fresh = root / expected_fixture.name
            subprocess.run(['.venv/bin/python', 'tools/evaluation_reference.py', '--model', entry.model,
                            'fixtures/evaluation-v1.json', str(fresh)], check=True)
            counts = Counts()
            compare(read_json(expected_fixture), read_json(fresh), counts)
            reports.append(ModelReport(entry.model, hashlib.sha256(expected_fixture.read_bytes()).hexdigest(), counts))
        capabilities = root / 'model-capabilities-v1.expected.json'
        generate(capabilities)
        counts = Counts()
        expected_capabilities = Path('fixtures') / capabilities.name
        compare(read_json(expected_capabilities), read_json(capabilities), counts)
        reports.append(ModelReport('capabilities', hashlib.sha256(expected_capabilities.read_bytes()).hexdigest(), counts))
    report = Path('target/reports/model-references.json')
    report.parent.mkdir(parents=True, exist_ok=True)
    report.write_text(json.dumps([asdict(item) for item in reports], indent=2) + '\n')
    print('PASS catalog model re-exports and independent small/large/capability references')


if __name__ == '__main__':
    main()
