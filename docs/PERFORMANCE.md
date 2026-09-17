# Performance measurements

These measurements describe scalar Rust inference on one machine. The direct comparison below shows that the current SpaRs implementation is slower than official spaCy for this corpus. Results are a starting point for optimization, not a promise about other workloads.

## Comparison with official spaCy

Both implementations processed the same `en_core_web_md` 3.8.0 pipeline on an Apple M3 Max with 96 GiB RAM. Rust used a release build. Python used spaCy 3.8.14 and Thinc 8.3.13. Each group reused its loaded model, ran one warmup pass, and then ran three measured passes. Processing was sequential, with Python numerical-library thread limits set to one. This compares individual calls, not batching.

| Group | SpaRs tokens/second | spaCy tokens/second | Result |
|---|---:|---:|---|
| Short documents | 1,135 | 3,486 | spaCy was 3.07 times faster |
| Long documents | 1,466 | 8,183 | spaCy was 5.58 times faster |

The short group measured 288 document calls and 3,264 tokens; the long group measured 6 calls and 13,440 tokens. Imports, model loading, corpus reading, document destruction, and JSON output were outside the inference timer. Python also materialized sentence, noun-chunk, and entity spans, which SpaRs computes during processing.

Model loading took 0.185 seconds for SpaRs and 0.644 seconds for spaCy in the short group, and 0.188 versus 0.628 seconds in the long group. Peak process memory was about 141 versus 401 MiB for short inputs and 163 versus 456 MiB for long inputs. Python memory includes the interpreter. These peaks include model loading; they do not measure inference allocations alone.

SpaRs uses scalar numerical kernels. Official spaCy uses compiled Cython and optimized numerical libraries. Profiling is needed to identify which operations account for the measured difference. Native Rust does not automatically imply faster inference.

To repeat the comparison after the model setup, run:

```sh
.venv/bin/python tools/compare_python.py
```

The [comparison report](../reports/python-comparison.json) records raw timings, workload counts, hardware, versions, source hashes, and the limits of the measurement. Each profile ran SpaRs before spaCy in separate processes. Operating-system caches were not cleared, and the synthetic long documents repeat one sentence. This single run does not establish performance on other hardware, text, or batch sizes.

## Earlier native baseline

The remaining measurements are the earlier baseline recorded in `reports/benchmark.json`. Its corpus predates the portable-path fixture update; its source hashes identify the implementation measured.

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
