# SpaRs implementation status

## English inference — verified 2026-09-16

SpaRs implements native Rust inference for `en_core_web_md` 3.8.0, using spaCy 3.8.14 and Thinc 8.3.13 as the reference. The supported pipeline includes tokenization, lexical features, neural encoding, tags, dependencies, attribute rules, lemmas, entities, sentences, noun chunks, and static vectors. The [compatibility inventory](COMPATIBILITY.md) lists remaining library features.

## Current evidence

| Check | Verified scope |
|---|---|
| Original full-pipeline suites | 44 documents / 1,035 tokens |
| Expanded domain and length coverage | 98 documents / 5,568 tokens, including documents of 896 and 3,584 tokens |
| Tokenizer boundary regressions | 24 documents / 120 tokens |
| Fresh post-fix comparison | 24 documents / 306 tokens; evaluated after the tokenizer correction |
| Combined full-pipeline agreement | 190 documents / 7,029 tokens; exact discrete outputs |
| Tokenizer cases | 4,057 cases / 14,737 tokens |
| Lexical properties | 4,374 cases |
| Regular-expression matching | 52,236 comparisons over 13,059 texts |
| Parser and entity action traces | 634 action choices, context IDs, and valid-action masks |
| New robustness checks | 9 malformed configurations, 4 damaged tensors, 27 malformed snapshots, 3 invalid UTF-8 offsets, and 28 processing results across repeated, batched, and concurrent calls |
| Executed Markdown examples | 1 README example and 4 developer-guide examples |

The reference comparisons require exact token annotations and spans. Floating-point calculations use the limits in [validation](VALIDATION.md). In the recorded intermediate tests, the maximum activation difference was 4.291534423828125e-6 and the maximum transition-score difference was 1.9073486328125e-5. Eight static-vector lookup cases and six similarity pairs also pass.

The [typed implementation verification report](../reports/verification-portable.json) records strict Python type checking, the typing policy, formatting, Clippy, Rust and Python tests, 2 source doc tests, the 5 executed Markdown examples, the separate native consumer, packaging, and a byte-identical model re-export. One source doc example is compile-only; it is not counted among the five executed Markdown examples. The consumer runs with no interpreters on its PATH.

## Strong typing

Runtime model configuration uses typed Rust records and enums instead of generic JSON values. Python tools use strict Pyright checking, named records, and checked conversions at external-data boundaries. Regression tests reject wrong field types, unknown operators, invalid null constraints, booleans used as integers, and typing-policy bypasses. The exported model format and frozen reference expectations remain unchanged.

## What the expanded evaluation found

The first 98-document run passed 97 documents. The remaining case contained `Wait—didn't`. SpaRs applied a contraction exception after splitting on the dash, while spaCy preserved the whole contraction there. The saved [first-run report](../reports/evaluation-v1-first-run.json) contains the input, expected output, and differences.

The correction follows spaCy's two-pass tokenizer. The second pass uses the same filtered rule set and matches the token sequence produced without exception handling. This also fixes related boundaries such as `He's-word`. Expected outputs stayed unchanged. All 98 cases and the 24 new boundary regressions now pass; the later 24-document comparison also passed without further runtime changes.

These are synthetic, project-authored examples. They broaden coverage but do not estimate accuracy on a random sample of real-world text. Once used, a comparison set becomes a regression suite rather than an untouched future test set.

## Model provenance

The official model wheel has SHA-256 `5e6329fe3fecedb1d1a02c3ea2172ee0fede6cea6e4aefb6a02d832dba78a310`. The exporter copies 69 F32 tensors, configuration, linguistic resources, and license notices. Source hashes verified against the official wheel are recorded in [source-lock.json](../reference/source-lock.json). For spaCy 3.8.14, wheel-shipped source was used because a source archive was unavailable at acquisition.

## Fixture generator limitation

The current lexical generator produces 5,367 cases and its regex generator produces 16,038 texts, while the frozen suites contain 4,374 lexical cases and 13,059 regex texts. This difference predates the typing changes: the previous and typed generators produce byte-identical outputs with the current exported resources. Existing fixture inputs and expectations remain frozen. Reconcile generator input provenance before claiming that every frozen suite can be regenerated from the current scripts.

## Performance and remaining work

See [performance measurements](PERFORMANCE.md) for hardware, loading time, throughput, and peak process memory. The implementation uses native CPU matrix kernels and processes documents sequentially. Measurements describe this machine and corpus; they do not promise a particular speed for other workloads.

The next library features are token matching and phrase matching, followed by dependency matching, document editing, broader serialization, additional pipelines and languages, and training. Follow the [quality process](QUALITY.md): every feature needs its own reference cases, failure tests, and runnable example before it is marked verified.
