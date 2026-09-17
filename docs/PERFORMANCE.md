# Performance measurements

These measurements describe the current scalar Rust implementation on one machine. They are a starting point for future optimization, not a promise about other workloads or a comparison with spaCy's speed.

## Recorded environment

- Machine: Apple M3 Max, 14 logical CPUs, 96 GiB RAM.
- System: macOS-27.0-arm64-arm-64bit.
- Rust: `rustc 1.98.0 (88d9e12ae 2026-08-18) (Homebrew)`; release build; one inference thread.
- Model: `en_core_web_md` 3.8.0 exported with spaCy 3.8.14 and Thinc 8.3.13.
- Corpus: the fixed `evaluation-v1.json` set. The short group has 96 documents, including empty and whitespace inputs. The long group has two repeated-sentence documents of 896 and 3,584 tokens.
- Procedure: a fresh process per group, one untimed pass to warm up, then three timed passes. File-system caches were not cleared. Model loading, corpus reading, and JSON output are outside the inference timer.

The [machine-readable report](../reports/benchmark.json) includes exact commands, source hashes, model and corpus hashes, and raw measurements.

## Results

| Measurement | Load only | Short documents | Long documents |
|---|---|---|---|
| Model load time | 0.354 s | 0.322 s | 0.319 s |
| Timed documents | — | 288 | 6 |
| Timed tokens | — | 3,270 | 13,440 |
| Inference time | — | 2.800 s | 9.557 s |
| Tokens per second | — | 1168 | 1406 |
| Documents per second | — | 102.86 | 0.63 |
| Median document time | — | 9.91 ms | 1590.67 ms |
| 95th-percentile document time | — | 12.53 ms | 2578.36 ms |
| Peak process memory | 313.94 MiB | 312.64 MiB | 335.31 MiB |

The median is the middle measured duration, or the average of the two middle durations when the sample count is even; the 95th percentile is a duration at or above 95% of the samples. With only six long-document measurements, that percentile is the slowest sample and is not a reliable estimate of production tail latency.

Peak memory is the process's highest resident memory usage, measured by the operating system. It includes loading and model storage, even for the inference groups. It is not an inference-only allocation count or proof that no memory leak exists. The smaller short-document peak reflects variation between separate processes; do not subtract these peaks to estimate processing memory.

The long documents repeat one sentence. They test length, not the full variety of long-form language. Different text, hardware, compiler versions, concurrency, and operating-system load can change these numbers.

## Repeat the measurement

After model setup and dependency fetching, run from the repository root on macOS or Linux:

```sh
.venv/bin/python tools/benchmark.py
```

The script builds the release measurement worker, runs the three groups, and writes `reports/benchmark.json`. It measures native Rust inference; Python only launches the worker and collects system information. The process needs permission to read CPU and RAM information. CI saves its own measurement report as a job artifact.

For a proposed optimization, first run the full acceptance suite. Then measure the old and new code on the same machine, with the same corpus, build mode, warmup, and number of passes. Report both results and keep annotation agreement exact. There is no automatic speed threshold yet; baseline variation needs to be measured before a trustworthy threshold can be chosen.
