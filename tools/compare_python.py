"""Compare reused native and official Python models on identical single-call inputs."""
import argparse
from dataclasses import asdict, dataclass
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import statistics
import subprocess
import tempfile
import time

from benchmark import command
from json_types import ModelMetadata, json_array, json_float, json_int, json_object, json_string, parse_json, read_json, validate_json
from report_paths import portable_json
from reference_types import Doc


class Options(argparse.Namespace):
    worker: bool = False
    profile: str = 'short'
    rounds: int = 3
    warmup: int = 1
    output: Path = Path('reports/python-comparison.json')


@dataclass(frozen=True)
class Measurements:
    load_seconds: float
    inference_seconds: float
    measured_documents: int
    measured_tokens: int
    median_document_seconds: float
    p95_document_seconds: float
    peak_process_rss_bytes: float

    def __post_init__(self) -> None:
        values = (self.load_seconds, self.inference_seconds, self.median_document_seconds, self.p95_document_seconds, self.peak_process_rss_bytes)
        if not all(math.isfinite(value) and value >= 0 for value in values):
            raise ValueError('Measurements must be finite and nonnegative')
        if self.inference_seconds <= 0 or self.measured_documents <= 0 or self.measured_tokens < 0:
            raise ValueError('Inference needs positive duration and document count')

    @property
    def tokens_per_second(self) -> float:
        return self.measured_tokens / self.inference_seconds


@dataclass(frozen=True)
class ProfileResult:
    native: Measurements
    python: Measurements
    native_speedup_over_python: float
    native_tokens_per_second: float
    python_tokens_per_second: float


@dataclass(frozen=True)
class ComparisonReport:
    model: str
    model_version: str
    reference_versions: dict[str, str]
    weights_sha256: str
    platform: str
    cpu: str
    logical_cpus: int | None
    ram_bytes: int
    rustc: str
    python: str
    build: str
    rounds: int
    warmup_passes: int
    profiles: dict[str, ProfileResult]
    revision: str
    working_tree_dirty: bool
    source_sha256: dict[str, str]
    command: list[str]
    limits: str


def reproduction_command(options: Options) -> list[str]:
    return ['.venv/bin/python', 'tools/compare_python.py', '--rounds', str(options.rounds), '--warmup', str(options.warmup), '--output', str(options.output)]


def compare_measurements(native: Measurements, python: Measurements) -> ProfileResult:
    if (native.measured_documents, native.measured_tokens) != (python.measured_documents, python.measured_tokens):
        raise ValueError('Native and Python workload counts differ')
    return ProfileResult(native, python, python.inference_seconds / native.inference_seconds, native.tokens_per_second, python.tokens_per_second)


def texts_for(profile: str) -> list[str]:
    corpus = json_object(read_json(Path('fixtures/evaluation-v1.json')))
    cases = [json_object(case) for case in json_array(corpus['cases'])]
    return [json_string(case['text']) for case in cases if (case['category'] == 'long') == (profile == 'long')]


def python_worker(options: Options) -> None:
    import spacy
    import thinc
    if spacy.__version__ != '3.8.14' or thinc.__version__ != '8.3.13':
        raise ValueError('Expected spaCy 3.8.14 and Thinc 8.3.13')
    start = time.perf_counter()
    model = spacy.load('en_core_web_md')
    load = time.perf_counter() - start
    if model.meta['version'] != '3.8.0':
        raise ValueError('Expected en_core_web_md 3.8.0')
    if model.pipe_names != ['tok2vec', 'tagger', 'parser', 'attribute_ruler', 'lemmatizer', 'ner']:
        raise ValueError('Unexpected reference pipeline')
    texts = texts_for(options.profile)

    def process(text: str) -> Doc:
        doc = model(text)
        # Rust materializes these spans during processing; consume Python's lazy views.
        tuple(doc.sents)
        tuple(doc.noun_chunks)
        tuple(doc.ents)
        return doc

    for _ in range(options.warmup):
        for text in texts:
            process(text)
    durations: list[float] = []
    tokens = 0
    for _ in range(options.rounds):
        for text in texts:
            start = time.perf_counter()
            doc = process(text)
            durations.append(time.perf_counter() - start)
            tokens += len(doc)
            del doc
    durations.sort()
    result = Measurements(load, sum(durations), len(durations), tokens, statistics.median(durations), durations[math.ceil(len(durations) * .95) - 1], 0)
    print(json.dumps(asdict(result)))


def run_worker(cmd: list[str]) -> Measurements:
    environment = dict(os.environ)
    for name in ('OMP_NUM_THREADS', 'OPENBLAS_NUM_THREADS', 'MKL_NUM_THREADS', 'VECLIB_MAXIMUM_THREADS', 'NUMEXPR_NUM_THREADS', 'BLIS_NUM_THREADS'):
        environment[name] = '1'
    with tempfile.TemporaryFile() as output:
        pid = os.posix_spawn(cmd[0], cmd, environment, file_actions=[(os.POSIX_SPAWN_DUP2, output.fileno(), 1)])
        _, status, usage = os.wait4(pid, 0)
        if os.waitstatus_to_exitcode(status) != 0:
            raise RuntimeError('Benchmark worker failed')
        output.seek(0)
        record = json_object(parse_json(output.read().decode('utf-8')))
    rss = usage.ru_maxrss * (1 if platform.system() == 'Darwin' else 1024)
    return Measurements(json_float(record['load_seconds']), json_float(record['inference_seconds']), json_int(record['measured_documents']), json_int(record['measured_tokens']), json_float(record['median_document_seconds']), json_float(record['p95_document_seconds']), rss)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--worker', action='store_true', help=argparse.SUPPRESS)
    parser.add_argument('--profile', choices=('short', 'long'), default='short')
    parser.add_argument('--rounds', type=int, default=3)
    parser.add_argument('--warmup', type=int, default=1)
    parser.add_argument('--output', type=Path, default=Path('reports/python-comparison.json'))
    options = Options()
    parser.parse_args(namespace=options)
    if options.rounds < 1 or options.warmup < 0:
        parser.error('Rounds must be positive and warmup nonnegative')
    if options.worker:
        python_worker(options)
        return
    if platform.system() not in ('Darwin', 'Linux'):
        parser.error('Memory measurement requires macOS or Linux')
    subprocess.run(['cargo', 'build', '--release', '--offline', '--example', 'measure'], check=True)
    profiles: dict[str, ProfileResult] = {}
    for profile in ('short', 'long'):
        native = run_worker(['target/release/examples/measure', 'assets/en_core_web_md-3.8.0', 'fixtures/evaluation-v1.json', profile, str(options.rounds), str(options.warmup)])
        python = run_worker(['.venv/bin/python', 'tools/compare_python.py', '--worker', '--profile', profile, '--rounds', str(options.rounds), '--warmup', str(options.warmup)])
        result = compare_measurements(native, python)
        speedup = result.native_speedup_over_python
        profiles[profile] = result
        print(f'{profile}: native {native.tokens_per_second:.1f} tokens/s; Python {python.tokens_per_second:.1f} tokens/s; native speedup {speedup:.3f}x', flush=True)
    cpu = command('sysctl', '-n', 'machdep.cpu.brand_string') if platform.system() == 'Darwin' else next((line.split(':', 1)[1].strip() for line in Path('/proc/cpuinfo').read_text().splitlines() if line.startswith('model name')), platform.machine())
    ram = int(command('sysctl', '-n', 'hw.memsize')) if platform.system() == 'Darwin' else os.sysconf('SC_PAGE_SIZE') * os.sysconf('SC_PHYS_PAGES')
    metadata = ModelMetadata.load(Path('assets/en_core_web_md-3.8.0/manifest.json'))
    report = ComparisonReport(
        model=metadata.model, model_version=metadata.model_version,
        reference_versions=metadata.versions, weights_sha256=metadata.weights_sha256,
        platform=platform.platform(), cpu=cpu, logical_cpus=os.cpu_count(), ram_bytes=ram,
        rustc=command('rustc', '--version'), python=platform.python_version(),
        build='release; matrixmultiply 0.3.10 Rust kernels; one processing thread; Python BLAS thread limits 1',
        rounds=options.rounds, warmup_passes=options.warmup, profiles=profiles,
        revision=command('git', 'rev-parse', 'HEAD'),
        working_tree_dirty=bool(command('git', 'status', '--porcelain')),
        source_sha256={str(path): hashlib.sha256(path.read_bytes()).hexdigest() for path in [Path('fixtures/evaluation-v1.json'), Path('tools/compare_python.py'), Path('examples/measure.rs'), Path('examples/measure/stats.rs'), Path('Cargo.lock'), *sorted(Path('src').rglob('*.rs'))]},
        command=reproduction_command(options),
        limits='One sequential native then Python run per profile; no batching or parallelism. Warm caches; OS caches not cleared. Inference excludes model load, imports, corpus reading, document destruction, and JSON output. Python materializes sentences, noun chunks and entities to match native work. Peak RSS includes interpreter and model loading. Load excludes Python import time. Synthetic corpus and single run do not establish general speed. Values below 1 mean Rust is slower.',
    )
    options.output.write_text(json.dumps(portable_json(validate_json(asdict(report)), Path.cwd()), indent=2) + '\n')


if __name__ == '__main__':
    main()
