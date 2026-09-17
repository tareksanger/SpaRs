"""Measure native inference in fresh processes on macOS or Linux."""
import argparse
import hashlib
import json
import os
import platform
import subprocess
import tempfile
from pathlib import Path
from typing import TypedDict
from json_types import ModelMetadata, parse_json, json_object, json_float, json_int


class Options(argparse.Namespace):
    rounds: int = 3
    warmup: int = 1
    output: Path = Path("reports/benchmark.json")


class LoadMeasurements(TypedDict):
    load_seconds: float
    peak_process_rss_bytes: float
    command: list[str]


class InferenceMeasurements(LoadMeasurements):
    corpus_documents: int
    rounds: int
    warmup_passes: int
    measured_documents: int
    measured_tokens: int
    inference_seconds: float
    documents_per_second: float
    tokens_per_second: float
    median_document_seconds: float
    p95_document_seconds: float


class BenchmarkReport(TypedDict):
    platform: str
    cpu: str
    logical_cpus: int | None
    ram_bytes: int
    rustc: str
    build: str
    model: str
    reference_versions: dict[str, str]
    weights_sha256: str
    corpus_sha256: str
    revision: str
    working_tree_dirty: bool
    profiles: dict[str, LoadMeasurements | InferenceMeasurements]
    source_sha256: dict[str, str]
    limits: str


def parse_measurements(text: str, profile: str, rss: float, cmd: list[str]) -> LoadMeasurements | InferenceMeasurements:
    value = json_object(parse_json(text))
    load = json_float(value["load_seconds"])
    if profile == "load":
        return {"load_seconds": load, "peak_process_rss_bytes": rss, "command": cmd}
    return {
        "load_seconds": load, "peak_process_rss_bytes": rss, "command": cmd,
        "corpus_documents": json_int(value["corpus_documents"]),
        "rounds": json_int(value["rounds"]),
        "warmup_passes": json_int(value["warmup_passes"]),
        "measured_documents": json_int(value["measured_documents"]),
        "measured_tokens": json_int(value["measured_tokens"]),
        "inference_seconds": json_float(value["inference_seconds"]),
        "documents_per_second": json_float(value["documents_per_second"]),
        "tokens_per_second": json_float(value["tokens_per_second"]),
        "median_document_seconds": json_float(value["median_document_seconds"]),
        "p95_document_seconds": json_float(value["p95_document_seconds"]),
    }


def command(*args: str) -> str:
    return subprocess.check_output(args, text=True).strip()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--rounds', type=int, default=3)
    parser.add_argument('--warmup', type=int, default=1)
    parser.add_argument('--output', type=Path, default=Path('reports/benchmark.json'))
    args = Options()
    parser.parse_args(namespace=args)
    if args.rounds < 1 or args.warmup < 0:
        parser.error('Use positive rounds and nonnegative warmup.')
    if platform.system() not in ('Darwin', 'Linux'):
        parser.error('Peak process memory measurement supports macOS and Linux.')
    subprocess.run(['cargo', 'build', '--release', '--offline', '--example', 'measure'], check=True)
    corpus = Path('fixtures/evaluation-v1.json')
    model = Path('assets/en_core_web_md-3.8.0')
    manifest = ModelMetadata.load(model/'manifest.json')
    profiles: dict[str, LoadMeasurements | InferenceMeasurements] = {}
    for profile in ('load', 'short', 'long'):
        cmd = ['target/release/examples/measure', str(model), str(corpus), profile, str(args.rounds), str(args.warmup)]
        with tempfile.TemporaryFile() as output:
            pid = os.posix_spawn(cmd[0], cmd, os.environ, file_actions=[(os.POSIX_SPAWN_DUP2, output.fileno(), 1)])
            _, status, usage = os.wait4(pid, 0)
            if os.waitstatus_to_exitcode(status) != 0:
                raise RuntimeError(f'benchmark failed: {cmd}')
            output.seek(0)
            text = output.read().decode("utf-8")
        rss = usage.ru_maxrss * (1 if platform.system() == 'Darwin' else 1024)
        measurements = parse_measurements(text, profile, rss, cmd)
        profiles[profile] = measurements
        print(f'PASS {profile}: {measurements}', flush=True)
    if platform.system() == 'Darwin':
        cpu = command('sysctl', '-n', 'machdep.cpu.brand_string')
        ram = int(command('sysctl', '-n', 'hw.memsize'))
    else:
        cpu = next((line.split(':', 1)[1].strip() for line in Path('/proc/cpuinfo').read_text().splitlines() if line.startswith('model name')), platform.machine())
        ram = os.sysconf('SC_PAGE_SIZE') * os.sysconf('SC_PHYS_PAGES')
    report: BenchmarkReport = {'platform':platform.platform(), 'cpu':cpu,'logical_cpus':os.cpu_count(), 'ram_bytes':ram,
              'rustc':command('rustc','--version'),'build':'release; scalar CPU kernels; one processing thread',
              'model':manifest.model+' '+manifest.model_version,'reference_versions':manifest.versions,
              'weights_sha256':manifest.weights_sha256,'corpus_sha256':hashlib.sha256(corpus.read_bytes()).hexdigest(),
              'revision':command('git','rev-parse','HEAD'),'working_tree_dirty':bool(command('git','status','--porcelain')),
              'profiles':profiles,
              'source_sha256':{str(p):hashlib.sha256(p.read_bytes()).hexdigest() for p in [*sorted(Path('src').glob('*.rs')), Path('examples/measure.rs'), Path('examples/measure/stats.rs'), Path('tools/benchmark.py'), Path('Cargo.lock')]},
              'limits':'Each profile uses a fresh process. Peak RSS includes loading, model storage and processing; it is not inference-only memory. OS file caches are not cleared. Timings exclude corpus reading and JSON output. Synthetic inputs are not representative of every application.'}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report,indent=2)+'\n')


if __name__ == '__main__':
    main()
