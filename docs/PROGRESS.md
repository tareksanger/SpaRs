# SpaRs implementation status

## English inference — verified 2026-09-16

SpaRs (`spars`) implements native Rust inference for `en_core_web_md` 3.8.0. The declared English acceptance suite passes; broader spaCy functionality remains under development. The [compatibility inventory](COMPATIBILITY.md) records the supported scope and missing capabilities.

### Verified current behavior

- Official en_core_web_md 3.8.0 wheel SHA256: `5e6329fe3fecedb1d1a02c3ea2172ee0fede6cea6e4aefb6a02d832dba78a310`.
- Official installed reference: spaCy 3.8.14 / Thinc 8.3.13 / Python 3.12.5. The model's declared spaCy range and spaCy's Thinc requirement are compatible; `uv pip check` passes. Full Python dependency versions and Rust Cargo locks exist.
- Reproducible official exporter: 69 F32 arrays, complete model resources, original config, Unicode tables, labels/actions and copied licensing metadata. A fresh export is byte-identical to the current model directory.
- Native algorithms: tokenizer, lexical flags/features/hashes, both tok2vecs, tagger, arc-eager parser, pseudo-projective recovery, attribute ruler, English lemmatizer, BILUO NER, sentence spans, noun chunks and static vector operations.
- Public APIs: immutable Send+Sync Model; owned Doc; checked TokenView/SpanView; byte/code-point/token newtypes; sequential streaming pipe; ordered stage controls; versioned validated native snapshots. Unsupported components do not silently run.
- Exact full-output parity: **44/44 documents, 1,035 tokens** (25 development, 10 frozen holdout, 9 later stress/regression cases; one case has 640 tokens).
- Exact tokenizer/features: **4,057/4,057 cases, 14,737 tokens**.
- Exact lexical properties: **4,374/4,374 cases**.
- Exact regex spans: **52,236/52,236 comparisons over 13,059 texts**.
- Exact parser/NER context IDs, validity masks, action choices: **634/634 steps**.
- Maximum intermediate activation error: 4.291534423828125e-6. Maximum transition score error: 1.9073486328125e-5. All satisfy predeclared tolerances; all required discrete annotations remain exact. Eight vector lookup and six similarity reference cases pass, plus doc-vector checks on the full-output corpus.
- `cargo fmt --check` (library and consumer), strict Clippy, **13 tests + 2 doc tests**, standalone offline consumer, and package verification all pass. Acceptance tests have **zero ignored tests**. Ordinary cargo test intentionally ignores asset-dependent tests; use the documented acceptance command.
- Separate consumer runs with an empty environment and nonexistent PATH. Cargo/runtime dependencies contain no Python/Node/WASM/network service layer.
- Local environment: macOS 27 arm64, rustc 1.98.0. This is validation metadata, not a benchmark. No throughput or memory-performance claims are made.

### Compatibility details

1. The exporter includes the official BASE_NORMS table and preserves model override order, which affects currency and dash normalization.
2. Pinned Unicode property/lowercase tables preserve Python lexical behavior independently of Rust's Unicode version. Anchored email matching preserves Python match semantics rather than regex search semantics.
3. Python regex shorthand classes are explicitly translated to pinned Unicode ranges rather than relying on a different regex engine's character classes.
4. The secondary WASM reference HTML has SHA-256 `8a60c2df6e88676970143455b72641c07d3a7e30b106517c3d4df178d8fa70c7`. Its output differs from official spaCy on a pseudo-projective dependency in development case 23, with an associated noun-chunk difference. Native output matches official Python. See [the WASM mismatch report](../reports/wasm-mismatches.json); no WASM model or executable is shipped.

### Source provenance and limits

- Source acquisition for spaCy 3.8.14 uses official wheel-shipped source verified against wheel RECORD: the GitHub source archive returned 404 and PyPI supplied no sdist at acquisition. Exact source hashes are recorded in [the source lock](../reference/source-lock.json).
- The reference environment pins typer 0.16.0 and click 8.1.8; dependency validation passes. The full dependency lock is [tools/reference-requirements.lock](../tools/reference-requirements.lock).
- [CI](../.github/workflows/ci.yml) exports official assets and executes the model-dependent acceptance tests.
- Scalar f32 baseline; sequential batching. Finite suites do not prove all-input parity. No linguistic-accuracy benchmark was conducted.

### Verification and next capabilities

Run the acceptance suite from the repository root after following the [setup instructions](../README.md):

```sh
.venv/bin/python tools/verify.py
```

[Validation documentation](VALIDATION.md) describes the suite and tolerances; [the verification report](../reports/verification.json) records command results.

Next broader-library implementation task: general token Matcher/PhraseMatcher with immutable document views, official-rule fixtures and explicit quantifier / match-order contracts. Then DependencyMatcher, retokenization, more serialization, additional pipelines/languages and training, each with independent acceptance suites. Preserve existing inputs and expected outputs when fixing failures. New cases belong in new regression fixtures; the used holdout is now a regression gate and cannot serve as a fresh future holdout.
