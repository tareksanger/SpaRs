"""Compare regenerated reference fixtures using the declared numerical limits."""
from dataclasses import asdict, dataclass, field
from enum import StrEnum
import hashlib
import importlib.metadata
import json
import math
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

from json_types import JsonValue, read_json

type JsonPath = tuple[str | int, ...]


class NumericalKind(StrEnum):
    EXACT = 'exact'
    ACTIVATION = 'activation'
    SCORE = 'score'
    VECTOR = 'vector'


@dataclass(frozen=True)
class Tolerance:
    absolute: float
    relative: float


TOLERANCES = {
    NumericalKind.EXACT: Tolerance(0.0, 0.0),
    NumericalKind.ACTIVATION: Tolerance(2e-5, 2e-5),
    NumericalKind.SCORE: Tolerance(2e-4, 2e-5),
    NumericalKind.VECTOR: Tolerance(2e-6, 2e-6),
}


@dataclass
class NumericalCounts:
    compared: int = 0
    maximum_absolute_difference: float = 0.0


@dataclass
class Counts:
    objects: int = 0
    arrays: int = 0
    discrete_values: int = 0
    numbers: dict[NumericalKind, NumericalCounts] = field(
        default_factory=lambda: {kind: NumericalCounts() for kind in NumericalKind})


@dataclass(frozen=True)
class Mismatch:
    path: str
    expected: str
    actual: str
    reason: str


class ReferenceMismatch(ValueError):
    def __init__(self, mismatch: Mismatch) -> None:
        self.mismatch = mismatch
        super().__init__(f'{mismatch.path}: {mismatch.reason}; expected {mismatch.expected}, actual {mismatch.actual}')


def path_text(path: JsonPath) -> str:
    return '$' + ''.join(f'[{part}]' if isinstance(part, int) else f'[{json.dumps(part)}]' for part in path)


def numerical_kind(path: JsonPath) -> NumericalKind:
    # Permit tolerance only at the known fixture fields, never merely because a
    # value is numeric or because an unrelated field has a familiar name.
    if len(path) >= 3 and path[0] == 'cases' and isinstance(path[1], int):
        if path[2] == 'tok2vec':
            return NumericalKind.ACTIVATION
        if path[2] == 'vector':
            return NumericalKind.VECTOR
    if len(path) >= 2 and isinstance(path[0], int):
        if path[1] in ('tok2vec_stages', 'ner_stages'):
            return NumericalKind.ACTIVATION
        if len(path) >= 4 and path[1] in ('parser', 'ner') and isinstance(path[2], int) and path[3] == 'scores':
            return NumericalKind.SCORE
    return NumericalKind.EXACT


def fail(path: JsonPath, expected: JsonValue, actual: JsonValue, reason: str) -> None:
    # repr keeps nonfinite values readable without writing invalid JSON numbers.
    raise ReferenceMismatch(Mismatch(path_text(path), repr(expected), repr(actual), reason))


def compare(expected: JsonValue, actual: JsonValue, counts: Counts, path: JsonPath = ()) -> None:
    if type(expected) is not type(actual):
        fail(path, expected, actual, 'value types differ')
    if isinstance(expected, dict) and isinstance(actual, dict):
        counts.objects += 1
        if expected.keys() != actual.keys():
            fail(path, [key for key in sorted(expected)], [key for key in sorted(actual)], 'object fields differ')
        for key, value in expected.items():
            compare(value, actual[key], counts, (*path, key))
    elif isinstance(expected, list) and isinstance(actual, list):
        counts.arrays += 1
        if len(expected) != len(actual):
            fail(path, len(expected), len(actual), 'array lengths differ')
        for index, (left, right) in enumerate(zip(expected, actual)):
            compare(left, right, counts, (*path, index))
    elif isinstance(expected, float) and isinstance(actual, float):
        if not math.isfinite(expected) or not math.isfinite(actual):
            fail(path, expected, actual, 'nonfinite floating-point value')
        kind = numerical_kind(path)
        tolerance = TOLERANCES[kind]
        difference = abs(actual - expected)
        if not math.isfinite(difference):
            fail(path, expected, actual, 'floating-point difference overflowed')
        numerical_counts = counts.numbers[kind]
        numerical_counts.compared += 1
        numerical_counts.maximum_absolute_difference = max(numerical_counts.maximum_absolute_difference, difference)
        limit = tolerance.absolute + tolerance.relative * abs(expected)
        if difference > limit:
            fail(path, expected, actual, f'absolute difference {difference!r} exceeds {limit!r} ({kind})')
    else:
        counts.discrete_values += 1
        if expected != actual:
            fail(path, expected, actual, 'discrete values differ')


@dataclass
class FixtureReport:
    file: str
    expected_sha256: str
    actual_sha256: str
    counts: Counts = field(default_factory=Counts)
    mismatch: Mismatch | None = None
    passed: bool = False


def main() -> None:
    root = Path(__file__).resolve().parent.parent
    expected = ['development.expected.json', 'stages.expected.json', 'hash.expected.json']
    reports: list[FixtureReport] = []
    failure: ReferenceMismatch | None = None
    with tempfile.TemporaryDirectory(prefix='spars-reference-check-') as temporary:
        work = Path(temporary)
        (work / 'fixtures').mkdir()
        shutil.copyfile(root / 'fixtures/development.json', work / 'fixtures/development.json')
        for script, arguments in [('fixtures.py', ['development']), ('stages.py', [])]:
            subprocess.run([sys.executable, str(root / 'tools' / script), *arguments], cwd=work, check=True)
        for name in expected:
            original = root / 'fixtures' / name
            generated = work / 'fixtures' / name
            report = FixtureReport(name, hashlib.sha256(original.read_bytes()).hexdigest(), hashlib.sha256(generated.read_bytes()).hexdigest())
            reports.append(report)
            try:
                compare(read_json(original), read_json(generated), report.counts)
                report.passed = True
            except ReferenceMismatch as error:
                report.mismatch = error.mismatch
                failure = failure or error
    output = root / 'target/reports/reference-regeneration.json'
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps({
        'passed': failure is None,
        'versions': {name: importlib.metadata.version(name) for name in ('spacy', 'thinc', 'numpy')},
        'tolerances': {kind: asdict(value) for kind, value in TOLERANCES.items()},
        'fixtures': [asdict(report) for report in reports],
    }, indent=2, allow_nan=False) + '\n')
    if failure is not None:
        raise failure
    print(f'PASS {len(expected)} regenerated reference fixtures: exact discrete outputs and declared float tolerances')


if __name__ == '__main__':
    main()
