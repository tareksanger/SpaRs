# How results are compared

The reference is official spaCy 3.8.14 with Thinc 8.3.13, Python 3.12.5, and `en_core_web_md` 3.8.0 on CPU. The disabled sentence-recognition component (`senter`) is excluded; the dependency parser supplies sentence boundaries.

## Exact results

Tokens, offsets, feature IDs, tags, morphology, lemmas, dependency heads and labels, entity labels, sentence boundaries, and noun chunks must match the saved reference exactly. Tests of parser and entity recognition steps also compare valid actions and selected actions. Any difference fails the relevant suite.

The full-pipeline suites now cover 190 documents and 7,029 tokens. [Implementation status](PROGRESS.md) breaks down the counts; [fixture provenance](../fixtures/README.md) explains where the inputs came from and which affected implementation. These checks measure agreement with spaCy, not whether spaCy's linguistic predictions are correct.

## Small numerical differences

A floating-point number is an approximation. Rust and the numerical library used by Thinc can add the same values in different orders, producing small differences. The allowed limits were set before acceptance testing:

| Values compared | Maximum allowed difference |
|---|---|
| Normalized intermediate neural outputs | `2e-5 + 2e-5 * abs(expected)` |
| Classifier and transition scores | `2e-4 + 2e-5 * abs(expected)` |
| Averaged static vectors | `2e-6 + 2e-6 * abs(expected)` |
| Stored static-vector rows | Exact equality |

All values must be finite and all dimensions must match. The difference is `abs(actual - expected)`. A different tag or action fails even if its underlying score is within tolerance.

Rust uses native F32 matrix kernels; Thinc uses BLAS. F32 epsilon is about 1.19e-7, and the longest dot product has 672 terms. A simple worst-case relative accumulation estimate is about 8e-5 before cancellation or normalization. The chosen limits allow small observed arithmetic differences while still requiring exact decisions. They are engineering limits, not a proof for every possible input. Near ties can still produce a failed comparison.

## Empty input

The complete pipeline accepts empty text. Calling Thinc's raw embedding model with only an empty document raises a shape error, so the intermediate fixtures record an empty array for that case. The official full pipeline is still called to obtain its annotations.

## Failure reports

Comparison reports retain case IDs or indices, input text, versions, expected and actual results, and counts. The expanded tests also check that the source input matches its saved checksum. Existing expected outputs stay fixed during repairs.

Run `.venv/bin/python tools/verify.py` after setup. The [quality process](QUALITY.md) explains the full checks, and [performance](PERFORMANCE.md) explains the separate speed and memory measurements.
