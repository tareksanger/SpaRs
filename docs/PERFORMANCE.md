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

## Where SpaRs spends time

A five-second native sample on the long-document group found maxout neural layers (which compute and select scores) in 2,974 of 4,057 profiler observations (about 73%), and attribute rules in 553 observations (about 14%). These are approximate shares of sampled time, not exact component durations. The two encoders together appeared in about 78% of samples; that includes the 73% for maxout blocks and must not be added to it.

The main difference is numerical execution. [The Rust encoder](../src/neural.rs) computes separate dot products for each token and output row. The pinned Thinc reference uses BLIS matrix multiplication across token rows. Both encoders are required by this model, so their cost is not accidental duplicated inference. Rust already precomputes parser and NER token features; those calculations also use small dot products.

[Attribute matching](../src/attributes.rs) scans 180 patterns from 179 rules. It computes lowercase text for each candidate even when the pattern has no lowercase condition or an earlier condition has already failed. Three regex conditions also compile their expressions during matching. These are avoidable costs. Allocating temporary vectors, cloning layer buffers, and recomputing lexical features are additional code-level candidates whose individual costs have not been measured.

The first optimization priorities are matrix kernels that process token rows together, followed by compiling attribute regexes when the model loads and avoiding repeated lowercase work. Each change must retain the exact annotation comparisons and existing numerical limits.

[The profiling report](../reports/profiling.json) also records independent pipeline-prefix timings for both groups. Repeat those measurements with:

```sh
cargo run --release --offline --example profile_stages -- assets/en_core_web_md-3.8.0 fixtures/evaluation-v1.json long 3
```

Use `short` in place of `long` for the short-document group. Each total includes all stages before the named stage. Subtracting adjacent totals estimates a component's cost, with timing noise; the tagger prefix includes the shared encoder, and the attribute prefix includes noun chunks. One warmup pass precedes three measured passes. Model loading and result destruction are excluded, and stage order rotates between passes. This helps diagnose the implementation; it does not predict the speedup of a proposed fix.

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
