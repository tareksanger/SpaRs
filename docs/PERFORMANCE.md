# Measure performance

Measure the checked-out implementation before making a speed claim. Loading time, inference time, and process memory answer different questions and must be reported separately. Keep generated measurements in ignored `target/reports/`; CI saves its measurements as downloadable run artifacts. Benchmark results are not committed to Git.

## Compare with official spaCy

After the model setup and dependency fetching in the [README](../README.md), run from the repository root:

```sh
.venv/bin/python tools/compare_python.py --output target/reports/python-comparison.json
```

The tool compares SpaRs and official spaCy using the same `en_core_web_md` 3.8.0 model and fixed `fixtures/evaluation-v1.json` corpus. Rust uses a release build. Processing is sequential, with Python numerical-library thread limits set to one. Each loaded model receives one warmup pass and three measured passes by default. This compares individual calls, not batching.

The short group contains 96 documents; three passes measure 288 document calls and 3,264 tokens. The long group contains two documents; three passes measure 6 calls and 13,440 tokens. Imports, model loading, corpus reading, document destruction, and JSON output are outside the inference timer. Python also materializes sentence, noun-chunk, and entity spans, which SpaRs computes during processing.

The generated report includes raw timings, workload counts, hardware, versions, source hashes, and measurement limits. Each profile runs SpaRs before spaCy in separate processes. Operating-system caches are not cleared, and the synthetic long documents repeat one sentence. One run does not establish performance on other hardware, text, or batch sizes. Repeat comparisons to check whether an apparent improvement exceeds normal timing variation.

## Measure native loading and inference

Run from the repository root on macOS or Linux:

```sh
.venv/bin/python tools/benchmark.py --output target/reports/benchmark.json
```

The script builds the release measurement worker and runs load-only, short-document, and long-document groups in separate processes. Python launches the native worker and collects system information; inference runs in Rust. The process needs permission to read CPU and RAM information.

The report records loading time, document and token throughput, per-document durations, and peak process memory. The median is the middle duration, or the average of the two middle durations when the sample count is even. The 95th percentile is a duration at or above 95% of the samples. With only six long-document measurements, that percentile is the slowest sample and does not reliably estimate production tail latency.

Peak memory is the process's highest resident memory usage, measured by the operating system. It includes loading and model storage, even for the inference groups. Python comparison peaks also include the interpreter. These values are not inference-only allocation counts or proof that no memory leak exists. Do not subtract peaks from separate processes to estimate processing memory.

## Profile pipeline stages

Measure successive pipeline prefixes to locate expensive stages:

```sh
cargo run --release --offline --example profile_stages -- assets/en_core_web_md-3.8.0 fixtures/evaluation-v1.json long 3
```

Use `short` in place of `long` for the short-document group. Each total includes all stages before the named stage. Subtracting adjacent totals estimates a component's cost, with timing noise; the tagger prefix includes the shared encoder, and the attribute prefix includes noun chunks. One warmup pass precedes three measured passes. Model loading and result destruction are excluded, and stage order rotates between passes. These measurements help locate expensive work; they do not predict the speedup of a proposed fix.

## Evaluate an optimization

Run the full acceptance suite to establish unchanged annotations before measuring speed. Compare the old and new implementations on the same machine, with the same model, corpus, release build settings, thread limits, warmup, and number of passes. Avoid competing test or benchmark processes. Record hardware, source and model versions, workload counts, loading time, inference throughput, and memory separately.

Keep exact annotation agreement and the existing numerical tolerances. Measure both short and long inputs, and add a relevant workload when the changed algorithm has different cost as input size grows. Report timing variation and memory tradeoffs alongside speed improvements. There is no automatic speed threshold yet; baseline variation needs to be measured before a trustworthy threshold can be chosen.
