# How results are compared

The reference is official spaCy 3.8.14 with Thinc 8.3.13, Python 3.12.5, and the supported English 3.8.0 models on CPU. The original suites use `en_core_web_md`; additional suites cover `sm` and `lg`. The disabled sentence-recognition component (`senter`) is excluded; the dependency parser supplies sentence boundaries.

## Exact results

Tokens, offsets, feature IDs, tags, morphology, lemmas, dependency heads and labels, entity labels, sentence boundaries, and noun chunks must match the saved reference exactly. Tests of parser and entity recognition steps also compare valid actions and selected actions. Any difference fails the relevant suite.

The original medium-model full-pipeline suites cover 190 documents and 7,029 tokens. [Implementation status](PROGRESS.md) breaks down the counts; [fixture provenance](../fixtures/README.md) explains where the inputs came from and which affected implementation. These checks measure agreement with spaCy, not whether spaCy's linguistic predictions are correct.

## Small numerical differences

A floating-point number is an approximation. Rust and the numerical library used by Thinc can add the same values in different orders, producing small differences. The allowed limits were set before acceptance testing:

| Values compared | Maximum allowed difference |
|---|---|
| Normalized intermediate neural outputs | `2e-5 + 2e-5 * abs(expected)` |
| Classifier and transition scores | `2e-4 + 2e-5 * abs(expected)` |
| Averaged static vectors | `2e-6 + 2e-6 * abs(expected)` |
| Contextual token, span, and document vectors | `2e-6 + 2e-6 * abs(expected)` |
| Stored static-vector rows | Exact equality |

All values must be finite and all dimensions must match. The difference is `abs(actual - expected)`. A different tag or action fails even if its underlying score is within tolerance.

Rust uses native F32 matrix kernels; Thinc uses BLAS. F32 epsilon is about 1.19e-7, and the longest dot product has 672 terms. A simple worst-case relative accumulation estimate is about 8e-5 before cancellation or normalization. The chosen limits allow small observed arithmetic differences while still requiring exact decisions. They are engineering limits, not a proof for every possible input. Near ties can still produce a failed comparison.

## Empty input

The complete pipeline accepts empty text. Calling Thinc's raw embedding model with only an empty document raises a shape error, so the intermediate fixtures record an empty array for that case. The official full pipeline is still called to obtain its annotations.

## Failure reports

Comparison reports retain case IDs or indices, input text, versions, expected and actual results, and counts. The expanded tests also check that the source input matches its saved checksum. Existing expected outputs stay fixed during repairs.

Run `.venv/bin/python tools/verify.py` after setup. Generated reports go to ignored `target/reports/`; they are not committed. CI uploads the reports produced by each run as downloadable artifacts, including available failure diagnostics. The [quality process](QUALITY.md) explains the full checks, and [performance](PERFORMANCE.md) explains the separate speed and memory measurements.

## Additional model releases

`tests/model_parity.rs` compares all token annotations, entities, sentences, noun chunks, and document vectors against separate small and large model references (98 documents / 5,568 tokens each). It also checks contextual token/span vectors against independent spaCy output, empty and unavailable vector states, and document snapshot round-trips and malformed tensors. `tests/model_loading.rs` checks identity-independent capability loading, declared component order against spaCy, missing stages, and rejected dependencies. Existing medium-model fixtures and tolerances remain unchanged.

The native installer tests compare complete converted manifests and tensor checksums for both additional packages, verify receipts across distinct models, test bounded archive reads, and reject a misplaced installation with another model's identity. `tools/check_models.py` re-exports and regenerates the new references; `tools/check_installer.py` runs all catalog models through the separate native consumer with an empty `PATH`. CI acquires all catalog models for acceptance and tests native downloads for the three supported English sizes. Locate results for the exact revision through the CI artifacts described in the quality guide.

Model discovery tests use isolated processes for environment overrides and check default cache rules, missing selections, malformed and oversized records, symlinks, model identity, and name/path precedence. Native installer and Node tests download from official local archives and then load by name. The Node suite also packs and installs the npm artifact and exercises its CLI without relying on repository TypeScript execution. Existing HTTP installer tests exercise transfer failures and checksum enforcement; loading itself remains offline.
