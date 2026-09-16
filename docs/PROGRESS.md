# Native spaCy port progress

## Handoff — 2026-09-16

The first native English inference implementation and its declared local
acceptance suite are complete. This does **not** complete the broader spaCy port.
Project name: SpaRs; Cargo package/import: `spars`. Local atomic commits
organize the implementation; no remote repository, release or registry package
is published.

### Verified current behavior

- Official en_core_web_md 3.8.0 wheel SHA256:
  `5e6329fe3fecedb1d1a02c3ea2172ee0fede6cea6e4aefb6a02d832dba78a310`.
- Official installed reference: spaCy 3.8.14 / Thinc 8.3.13 / Python 3.12.5.
  The model's declared spaCy range and spaCy's Thinc requirement are compatible;
  `uv pip check` passes. Full Python dependency versions and Rust Cargo locks exist.
- Reproducible official exporter: 69 F32 arrays, complete model resources,
  original config, Unicode tables, labels/actions and copied licensing metadata.
  A fresh export is byte-identical to the current model directory.
- Native algorithms: tokenizer, lexical flags/features/hashes, both tok2vecs,
  tagger, arc-eager parser, pseudo-projective recovery, attribute ruler, English
  lemmatizer, BILUO NER, sentence spans, noun chunks and static vector operations.
- Public APIs: immutable Send+Sync Model; owned Doc; checked TokenView/SpanView;
  byte/code-point/token newtypes; sequential streaming pipe; ordered stage controls;
  versioned validated native snapshots. Unsupported components do not silently run.
- Exact full-output parity: **44/44 documents, 1,035 tokens** (25 development,
  10 frozen holdout, 9 later stress/regression cases; one case has 640 tokens).
- Exact tokenizer/features: **4,057/4,057 cases, 14,737 tokens**.
- Exact lexical properties: **4,374/4,374 cases**.
- Exact regex spans: **52,236/52,236 comparisons over 13,059 texts**.
- Exact parser/NER context IDs, validity masks, action choices: **634/634 steps**.
- Maximum intermediate activation error: 4.291534423828125e-6. Maximum transition
  score error: 1.9073486328125e-5. All satisfy predeclared tolerances; all required
  discrete annotations remain exact. Eight vector lookup and six similarity
  reference cases pass, plus doc-vector checks on the full-output corpus.
- `cargo fmt --check` (library and consumer), strict Clippy, **13 tests + 2 doc
  tests**, standalone offline consumer, and package verification all pass.
  Acceptance tests have **zero ignored tests**. Ordinary cargo test intentionally
  ignores asset-dependent tests; use the documented acceptance command.
- Separate consumer runs with an empty environment and nonexistent PATH.
  Cargo/runtime dependencies contain no Python/Node/WASM/network service layer.
- Local environment: macOS 27 arm64, rustc 1.98.0. This is validation metadata,
  not a benchmark. No throughput or memory-performance claims are made.

### Mismatches investigated

1. BASE_NORMS missing from the initial exporter caused currency/dash annotation
   differences. Exported the official table with correct model override order.
2. Rust Unicode classification/lowercase version differences and regex search vs
   Python match semantics caused lexical differences. Exported pinned Unicode
   property/lowercase tables and anchored email matching; all cases now pass.
3. Python regex shorthand classes are explicitly translated to pinned Unicode
   ranges rather than relying on a different regex engine's character classes.
4. Secondary WASM hash matches supplied `8a60c2df...70c7`. Its output differs from
   official spaCy on a pseudo-projective dependency in development case 23, with
   an associated noun-chunk difference. Native output matches official Python.
   See reports/wasm-mismatches.json; no WASM model or executable is shipped.

### Prerequisite issues and limits

- GitHub's v3.8.14 source archive returned 404 and PyPI supplied no sdist for that
  release. Resolved by reading official wheel-shipped source, verifying it against
  wheel RECORD, and recording exact source hashes in reference/source-lock.json.
- Initial reference import failed with typer 0.27.2 missing click. Resolved with
  pinned typer 0.16.0 and click 8.1.8; dependency validation passes.
- The earlier provisional package name was checked before this rename. That
  check does not establish availability of `spars`; registry availability must
  be checked before publishing. The selected project name is SpaRs.
- GitHub Actions workflow is defined to export assets and actually execute every
  model test, but **remote CI has not run** because no remote was published.
- Scalar f32 baseline; sequential batching. Finite suites do not prove all-input
  parity. No linguistic-accuracy benchmark was conducted.

### Resume / next task

Read README.md, docs/COMPATIBILITY.md and reports/verification.json. Run:

```
.venv/bin/python tools/verify.py
```

Next broader-library implementation task: general token Matcher/PhraseMatcher
with immutable document views, official-rule fixtures and explicit quantifier /
match-order contracts. Then DependencyMatcher, retokenization, more serialization,
additional pipelines/languages and training, each with independent acceptance
suites. Preserve existing inputs and expected outputs when fixing failures.
New cases belong in new regression fixtures; the used holdout is now a regression
gate and cannot serve as a fresh future holdout.

## 2026-09-16 naming and commit organization

Renamed the project to SpaRs and the Cargo package/import to `spars`. The on-disk
checkout remains SpaCyRust so the active workspace path stays valid. Initial
commits separate reference/export resources, native runtime, and release validation.

Rename verification: all acceptance gates pass with `spars`, including the
standalone consumer, 13 tests + 2 doc tests, package verification and byte-identical
model re-export. Initial reference commit: aeef66e; native runtime: b4b8407.
