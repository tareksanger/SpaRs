# SpaRs implementation status

## English inference

SpaRs implements native Rust inference for `en_core_web_sm`, `en_core_web_md`, and `en_core_web_lg` 3.8.0, using spaCy 3.8.14 and Thinc 8.3.13 as the reference. The supported pipeline includes tokenization, lexical features, neural encoding, tags, dependencies, attribute rules, lemmas, entities, sentences, noun chunks, static vectors for `md`/`lg`, and contextual vectors for `sm`. The [compatibility inventory](COMPATIBILITY.md) lists remaining library features.

## Current evidence

| Check | Verified scope |
|---|---|
| Original md full-pipeline suites | 44 documents / 1,035 tokens |
| Expanded domain and length coverage | 98 documents / 5,568 tokens, including documents of 896 and 3,584 tokens |
| Tokenizer boundary regressions | 24 documents / 120 tokens |
| Fresh post-fix comparison | 24 documents / 306 tokens; evaluated after the tokenizer correction |
| Combined full-pipeline agreement | 190 documents / 7,029 tokens; exact discrete outputs |
| Tokenizer cases | 4,057 cases / 14,737 tokens |
| Lexical properties | 4,374 cases |
| Regular-expression matching | 52,236 comparisons over 13,059 texts |
| Parser and entity action traces | 634 action choices, context IDs, and valid-action masks |
| New robustness checks | 9 malformed configurations, 4 damaged tensors, 27 malformed snapshots, 3 invalid UTF-8 offsets, and 28 processing results across repeated, batched, and concurrent calls |
| Dependency traversal | 98 documents / 5,568 tokens compared with official children, ancestors, subtree order, and sentence spans |
| Token Matcher | 72 documents / 5,533 rule registrations / 7,253 ordered matches against official spaCy |
| Additional sm/lg full-pipeline suites | 98 documents / 5,568 tokens each; separate frozen official references, exact annotations and vector tolerances in `crates/spars/tests/model_parity.rs` |
| Model capability and order checks | Independent model identity, declared pipeline order, rejected dependencies, and independent contextual token/span vectors in `crates/spars/tests/model_loading.rs` and `crates/spars/tests/model_parity.rs` |
| Executed Rust Markdown examples | 1 README example and 9 guide examples |
| Node-API binding | 190 documents / 7,029 tokens, all mapped annotations and offset units; eight static-vector lookup cases; typed API and async lifecycle/error tests |
| Executed TypeScript guide examples | 3 examples checked and run from Markdown |

The reference comparisons require exact token annotations and spans. Floating-point calculations use the limits in [validation](VALIDATION.md). Each run records the observed numerical differences in its generated reports. Eight static-vector lookup cases and six similarity pairs also pass.

The [acceptance script](../tools/verify.py) checks strict Python and TypeScript types, the typing policy, formatting, Clippy, Rust, Python and Node tests, 2 source doc tests, 10 Rust and 3 TypeScript examples executed from Markdown, the separate native consumer, packaging, and a byte-identical model re-export. One source doc example is compile-only; it is not counted among the executed Markdown examples. The Rust consumer runs with no interpreters on its PATH. Run `.venv/bin/python tools/verify.py` after setup to generate evidence under ignored `target/reports/`. CI uploads these files as run artifacts; see the [quality process](QUALITY.md).

## Strong typing

Runtime model configuration uses typed Rust records and enums instead of generic JSON values. Python tools use strict Pyright checking, named records, and checked conversions at external-data boundaries. Regression tests reject wrong field types, unknown operators, invalid null constraints, booleans used as integers, and typing-policy bypasses. Legacy v1 assets and existing frozen reference expectations remain unchanged; additional models use v2.

## What the expanded evaluation found

The first 98-document run passed 97 documents. The remaining case contained `Wait—didn't`. SpaRs applied a contraction exception after splitting on the dash, while spaCy preserved the whole contraction there. The fixed input and expected output remain in the [evaluation fixtures](../fixtures/README.md), together with the boundary regressions that exercise the correction.

The correction follows spaCy's two-pass tokenizer. The second pass uses the same filtered rule set and matches the token sequence produced without exception handling. This also fixes related boundaries such as `He's-word`. Expected outputs stayed unchanged. All 98 cases and the 24 new boundary regressions now pass; the later 24-document comparison also passed without further runtime changes.

These are synthetic, project-authored examples. They broaden coverage but do not estimate accuracy on a random sample of real-world text. Once used, a comparison set becomes a regression suite rather than an untouched future test set.

## Model provenance

The official `en_core_web_md` 3.8.0 wheel has SHA-256 `5e6329fe3fecedb1d1a02c3ea2172ee0fede6cea6e4aefb6a02d832dba78a310`. Other supported releases are pinned in [the model catalog](../models/catalog.json). The medium-model exporter copies 69 F32 tensors, configuration, linguistic resources, and license notices. Source hashes verified against the official wheel are recorded in [source-lock.json](../reference/source-lock.json). For spaCy 3.8.14, wheel-shipped source was used because a source archive was unavailable at acquisition.

## Fixture generator limitation

The current lexical generator produces 5,367 cases and its regex generator produces 16,038 texts, while the frozen suites contain 4,374 lexical cases and 13,059 regex texts. This difference predates the typing changes: the previous and typed generators produce byte-identical outputs with the current exported resources. Existing fixture inputs and expectations remain frozen. Reconcile generator input provenance before claiming that every frozen suite can be regenerated from the current scripts.

## Performance and remaining work

See [performance measurements](PERFORMANCE.md) for commands to measure loading time, throughput, and peak process memory. The implementation uses native CPU matrix kernels and processes documents sequentially. Each measurement applies to its recorded machine, source version, and corpus; it does not promise a particular speed for other workloads.

Dependency traversal, sentence access, the [typed DependencyMatcher](DEPENDENCY_MATCHER.md), and the [Token Matcher](TOKEN_MATCHER.md) are implemented. Token Matcher supports shared text and annotation conditions with repetition and overlapping results. DependencyMatcher verification covers all 20 relationships and its declared token-condition subset, including ordered results and supplementary morphology regressions. Wider matcher compatibility remains partial. The next capabilities, in priority order, are:

1. Model extensibility for all official pretrained pipelines, following the plan below. Catalog-selected installation, capability-declared loading, and configurable execution order are implemented for the English CNN family; additional component sets, languages and architectures remain planned.
2. PhraseMatcher: reusable phrase patterns with explicit attribute selection, overlap behavior, and result ordering. This remains the next matcher feature.

Document editing, broader serialization, custom-trained pipeline loading, and training remain later work. Follow the [quality process](QUALITY.md): every feature needs its own reference cases, failure tests, performance review, and runnable example before it is marked verified.

The separate [Node binding](NODE.md) exposes loading, processing, batches, ordered stages, static word vectors, and explicit native model downloads. Further binding work includes matcher and traversal APIs, document/span vectors and similarities, and binary packaging for supported platforms. Browser WASM remains unimplemented. These bindings reuse the native library; they do not change the broader spaCy compatibility backlog.

## Model extensibility plan

The target is native loading and inference for all official spaCy pretrained pipelines across languages, sizes, and architectures. Official releases must be identified by version and source provenance; this target does not promise automatic compatibility with unknown future release formats. Custom-trained pipeline loading is future scope. Its model data and additional native components should fit the same contracts, without a second loader or a requirement that every model originate in the official catalog. Executing arbitrary Python components and training models are separate capabilities, not prerequisites for this target.

The [v2 loader](../crates/spars/src/model.rs) checks explicit capability declarations independently of model identity. [Pipeline execution](../crates/spars/src/pipeline.rs) follows declared component order and validates dependencies; shared and NER encoders allow static vectors to be absent. The [installer](../installer/src/recipe.rs) selects conversion recipes by typed catalog identity and applies per-release limits. The three English sizes have separate reference coverage. Remaining constraints include a manifest that still requires all six English component configurations, English language behavior, and the implemented CNN and greedy-transition architectures. These must be extended before other pipeline families can execute.

### Separate acquisition, configuration, and execution

The architecture separates three responsibilities: decode model assets, check that the runtime implements the declared capabilities, and construct an executable pipeline. Track release-specific reference evidence separately from those capability checks. A catalog entry supplies identity, source locations, checksums, resource provenance, and archive limits; it must not define inference behavior through model-name branches. A new official model using implemented capabilities should need model data, acquisition metadata, and reference evidence without a runtime source change.

Use typed, versioned configuration for component factories (constructors for pipeline stages) and neural architectures, including tensor references and operation-specific validation. Resolve component instances from their declared order and dependencies on shared encoders, which are networks that produce token features for multiple components. Do not assume every model has a tagger, parser, lemmatizer, and NER in the English medium sequence. Language-specific tokenization, lexical rules, lemmatization, and noun chunks need their own resources or native implementations. Missing capabilities must identify the component, architecture, language behavior, or format that is unsupported; never silently substitute English behavior or omit a requested component.

Keep registration of native component implementations separate from the model catalog. Future custom-trained pipelines built from supported components should reuse the same loading and execution contracts. Additional custom behavior will require a registered native implementation and its validation contract. This extension point does not require a Python fallback, loading separately compiled native plugins, or a training API now. Loaded models remain immutable and reusable, with state owned by each processing call.

### Implementation order and acceptance

1. Inventory the components, architectures, language resources, and serialization requirements of version-pinned official pipelines. Group work by shared capabilities and record gaps in the compatibility inventory. Include transformer and language-specific families; the English sizes are initial acceptance cases, not the complete scope.
2. Separate typed model identity and acquisition metadata from reusable conversion and runtime configuration. Introduce the required versioned format changes with an explicit path for existing v1 assets. Preserve checksum, tensor, resource, and unsupported-configuration checks. Test two distinct model identities without duplicating conversion or inference logic.
3. Construct pipeline execution from typed component configurations and dependencies. Add independent cases for shared encoders, missing or reordered components, unavailable annotations, and invalid dependencies. Preserve the existing English pipeline's reference outputs and performance while removing its role as the universal pipeline shape.
4. Validate `en_core_web_sm`, `en_core_web_md`, and `en_core_web_lg` as initial real model cases, implementing their actual declared requirements, including models without static vectors. Each needs its own official reference outputs. Keep the existing frozen `md` suite; measure installation, loading, peak memory, and inference separately for each size.
5. Implement the remaining shared components, neural architectures, and language behavior identified by the official-model inventory, including transformer pipelines. Extend native conversion, reference tests, and consumer examples for each family. The all-official-pipelines target remains incomplete until every release in the declared version-pinned inventory passes its required checks.

Use the [quality process](QUALITY.md) for each bounded change. Synthetic configuration tests establish validation and extension mechanics, not compatibility with a real model. New weights and languages require their own reference evidence even when they reuse existing runtime code. The following installation requirements also apply to additional models.

## Versioned model installation plan

The first target is the official `en_core_web_md` 3.8.0 package. A native installer will download and verify official assets, convert them into the supported native format, and return an installed model location. Installation must work without Python, Node, WASM, or subprocess-based conversion. Loading and processing remain offline. No project-hosted model service or release attachment is required. Python remains available to maintainers for generating reference data and checking conversion.

The [native installer](MODEL_INSTALLATION.md) implements catalog-selected package conversion, downloads, verification, listing, and repeat-safe installation. Its model-dependent test compares every manifest field and all tensor bytes with the Python export. The versioned identities and catalog establish the boundary for updates; sm/md/lg 3.8.0 have native conversion comparisons; automatic update discovery and removal commands remain unimplemented. The milestones below retain the acceptance requirements for extending this work.

### 1. Define model identity and account for every resource

Add typed installation records that distinguish the upstream model name/version, official archive checksum, conversion recipe revision, linguistic-resource revision, and native format version. A conversion recipe is the set of rules used to turn official package data into native assets. Changing a recipe must not overwrite an existing installation of the same upstream model. Record the supported spaCy/Thinc versions, architectures, source URLs, notices, and expected archive sizes or limits. The initial compatibility catalog ships with the installer and lists explicitly supported releases; it does not automatically trust or install the latest upstream version.

Trace every field written by `tools/export.py` and `tools/lexical.py` to its source. Classify it as stored package data, derived configuration, or pinned language data that must ship separately with the installer. Small language tables may be generated during development and distributed with their licenses and checksums. Do not assume the model archive contains Python-generated Unicode tables, normalization defaults, or symbol definitions.

Acceptance: one reviewed mapping covers every current manifest field and tensor; typed records reject malformed digests, invalid versions, and unsupported combinations. Identify every required binary format from pinned upstream readers before choosing decoding dependencies. This is the first implementation increment.

### 2. Convert a local official package entirely in Rust

Read an already-downloaded, checksum-verified package without executing its contents. Implement only the stored formats needed by the pinned pipeline. Recover weights, configuration, ordered actions and labels, tokenizer rules, lookup tables, and vector mappings. Combine them with the pinned language resources to produce the existing native manifest and SafeTensors assets. Preserve applicable notices and source provenance. Reject unsupported architectures and malformed arrays before creating a usable installation.

Acceptance: compare tensor names, shapes, values, configuration, and linguistic resources against the Python exporter, then run the existing stage and full-pipeline parity suites. Require exact data equality where the representation is unchanged; differences in JSON layout or provenance fields need explicit, tested rules. Preserve existing floating-point tolerances. A separate native program converts the local package and runs inference with interpreters absent and network access disabled.

### 3. Add explicit downloads and reliable installation

Keep networking and archive handling outside normal inference and consumer builds, in an optional installer target or a separate tooling crate if needed. Download from the catalog's official sources with bounded retries, timeouts, and streaming checksum verification. Support a local archive for disconnected installation. Validate archive entries and extraction limits, including path traversal, duplicate destinations, links, and oversized contents.

Build in a temporary directory on the destination filesystem. Validate the complete output and its inventory before committing it with an atomic rename, which makes the complete model visible in one filesystem operation. Use locking or an equivalent mechanism so concurrent installs cannot corrupt each other. An interrupted or failed installation must leave existing models usable; rerunning the same installation must be safe.

Acceptance: controlled download tests cover truncation, bad checksums, unavailable sources, interruption, malformed archives, concurrent attempts, and repeat installation. A pinned official download is an explicit end-to-end check; repeatable failure tests use local fixtures rather than public-network failures.

### 4. Support explicit updates and rollback

Store supported releases side by side, keyed by complete installation identity. Applications select an exact installed version or path. Installing a newer release does not switch an application's model. Provide listing, verification, and explicit removal; rollback means selecting the retained prior installation. Do not delete older models automatically or mutate a loaded model. A new supported release may initially require an installer/library update to obtain its reviewed catalog entry and conversion recipe.

Legacy v1 loading pins the original model identity. V2 loading uses typed capability contracts, while the installer catalog and frozen reference suites retain release-specific evidence. Preserve capability and reference-version validation; broad version ranges are not evidence of compatibility. New weights on a supported architecture still require reference evaluation. Different architectures or resource semantics may require reusable Rust implementations and a new native format version.

Acceptance: lifecycle tests install two distinct identities, select each deterministically, reject unsupported combinations, and preserve the old installation through failed updates. Synthetic records can test lifecycle mechanics but cannot establish compatibility with a real second model. Declare a second upstream release supported only after its own pinned reference and parity suite pass.

### 5. Complete the consumer workflow and quality gates

Add a plain-English installation guide with commands and a separate Rust consumer example that actually run. Extend CI to execute native local-package conversion and offline inference without silently skipping model tests. Keep Python reference/export jobs separate from the native installation acceptance job. Record checksums and fixtures in Git; generated mismatch and performance reports remain under ignored `target/reports/`.

Acceptance: formatting, Clippy, typing, unit tests, malformed-input tests, reference comparisons, documentation examples, and package dry runs pass. Review installation security boundaries, source fidelity, test quality, and documentation independently. Measure download, conversion, disk usage, peak memory, and subsequent loading separately; check inference against the unchanged baseline if loader/runtime code changes. Public documentation must still distinguish this pinned installer from support for arbitrary spaCy packages.

Name-based loading and explicit downloads share `SPARS_MODEL_DIR` and operating-system cache defaults across Rust, Node, and the `spars` CLI. `crates/spars/tests/model_store.rs`, `installer/tests/project.rs`, and `bindings/node/tests/models.test.mts` cover malformed selection records, path precedence, environment discovery, native installation, missing-model errors, and an installed npm package. Downloading is separate from loading, and path overrides do not persist project configuration.
