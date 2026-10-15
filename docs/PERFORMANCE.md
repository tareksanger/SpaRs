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

## Phrase dictionaries

Build `cargo build --release --offline --example phrase_benchmark`, then run `target/release/examples/phrase_benchmark phrase shared 1000 4096 16`. The arguments select engine (`phrase`, the existing `token` matcher with text conditions, `lower` for Token Matcher lowercase conditions, or `lower-upper` for lowercase conditions on uppercase input that must be lowercased per token), scenario (`independent`, `shared`, `miss`, `dense`, `pruned`), pattern count, document token count and phrase length. Dense patterns use repeated Unicode tokens. The pruned case registers many labels on one phrase and removes all but one before searching; it reports pruning separately. Setup creates validated documents before timing; reports separate pattern registration, cold symbol initialization, the first matching call, five matching calls after two warmups, removal, and output count. Matching timings include result destruction. Wrap each process with the platform's peak-memory tool (for example `command time -l` on macOS). Process memory includes input preparation, matcher storage and outputs; it is not isolated matcher heap usage.

Compare the same workload and release binary, repeat processes, alternate engines and avoid concurrent builds. Token Matcher is a functional baseline for exact phrase patterns, not the same result-order contract. For algorithm changes preserve the previous PhraseMatcher binary too. Large output counts and long shared prefixes require separate workloads; a short no-match case cannot establish overall matching performance.

## Token Matcher rules

Build `cargo build --release --offline --example token_matcher_benchmark`, then run `target/release/examples/token_matcher_benchmark 10000 1000 indexed 5`. The arguments are the document token count, the number of rules (one pattern each), the pattern mix and the number of timed matching calls. Documents repeat a 200-word vocabulary. The `indexed` mix uses patterns whose first item requires an exact `Text` or `Lower` value (one-token, two-token and `In` with `OneOrMore` patterns); `unindexed` uses patterns that start with `NotIn`, `Optional`, `Negated` or `ZeroOrMore` items, which are tried at every token; `mixed` alternates the two. The output reports registration time, the median, minimum and maximum of the matching calls after one warmup call, the time to remove every tenth rule, the match counts, and a hash of the ordered matches before and after removal. Two builds produce identical matches when their hashes agree on the same machine. Wrap the process with `command time -l` on macOS for its peak memory.

To evaluate a change to the matcher, copy the release binary built from the earlier revision before editing, build the new one, and run both alternately on the same arguments over several rounds. Matching time grows with the document length multiplied by the number of patterns that can start at each token, so cover small and large documents and pattern counts and each mix. Workloads with millions of matches also measure output allocation, and peak memory for them varies with the system allocator's reuse of freed memory between calls.

The following results compare the engine that started every pattern at every token with the first-token index described in the [Token Matcher guide](TOKEN_MATCHER.md#scope-and-cost). They were recorded on an Apple M3 Max (14 cores, 96 GB RAM, macOS 27.0.1) with rustc 1.93.0 and the release profile, in three alternating rounds; each value is the median of the per-round medians, with the range of those medians in parentheses. Matching used five timed calls, or three when tokens multiplied by rules exceeded ten million. Every workload produced identical match hashes for both builds.

| Mix | Tokens | Rules | Matches | Before (ms) | After (ms) |
|---|---:|---:|---:|---:|---:|
| indexed | 1,000 | 100 | 498 | 1.56 (1.55–1.74) | 0.14 (0.13–0.14) |
| indexed | 1,000 | 10,000 | 49,900 | 174 (170–189) | 7.8 (7.7–9.2) |
| indexed | 10,000 | 1,000 | 50,010 | 174 (173–194) | 7.9 (7.7–8.5) |
| indexed | 100,000 | 100 | 50,015 | 155 (151–158) | 14.1 (12.1–14.2) |
| indexed | 100,000 | 10,000 | 5,000,500 | 18,485 (17,137–18,677) | 1,128 (1,011–1,311) |
| mixed | 10,000 | 1,000 | 50,197 | 298 (297–300) | 243 (243–250) |
| mixed | 100,000 | 1,000 | 501,960 | 2,935 (2,927–3,005) | 2,504 (2,396–2,528) |
| unindexed | 10,000 | 1,000 | 50,390 | 399 (394–401) | 396 (386–397) |
| unindexed | 10,000 | 10,000 | 503,900 | 4,206 (4,142–4,512) | 4,138 (3,930–6,226) |

Indexed patterns matched 11 to 22 times faster. Mixed workloads improved by about a fifth, because their unindexed half still runs at every token. Unindexed workloads were within the measured variation. Registering 10,000 indexed rules took 3.7 to 4.0 ms instead of 3.5 to 3.7 ms, and removing 1,000 of 10,000 rules took 45 to 49 ms instead of 82 to 86 ms, because removal no longer hashes every later rule name. A counting allocator showed about 114 KB of extra matcher storage for 10,000 one-token rules over 200 distinct values, and the same peak matching allocation to within 0.4 MB. Peak process memory agreed within 6 MB except for the five-million-match workload, which measured 1.0 GB before and 1.6 GB after in these runs. With a counting allocator, both builds held the same live memory after every call, and either build reached 1.6 GB when its calls ran back to back, so that difference reflects the system allocator's reuse of freed memory rather than matcher storage.
