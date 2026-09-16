# SpaRs

Experimental, standalone Rust NLP library using **official spaCy pretrained
weights**, with native Rust tokenization and inference. No Python, Node, WASM,
subprocess, network client, or build-time model download exists in the runtime.
The project is named **SpaRs**, with Cargo package and Rust import name `spars`.

The implemented target is **en_core_web_md 3.8.0**, exported with **spaCy 3.8.14 /
Thinc 8.3.13**. It includes tokenization, lexical features, both tok2vec networks,
tags, dependencies, attribute rules, lemmas, NER, sentences, English noun chunks,
and static vectors. Local validation passes on 44 full-pipeline documents /
1,035 tokens, 4,057 tokenizer cases and 634 transition steps; all discrete
outputs agree with the official reference. This is a bounded English inference milestone, **not a port
of the entire spaCy library**. See [compatibility](docs/COMPATIBILITY.md),
[progress](docs/PROGRESS.md), and [validation](docs/VALIDATION.md).

## Use from Rust

Use a local checkout as a Cargo path dependency:

```toml
[dependencies]
spars = { path = "/path/to/SpaRs" }
```

```rust,no_run
use spars::{Model, TokenIndex};
# fn main() -> Result<(), Box<dyn std::error::Error>> {
let nlp = Model::load("assets/en_core_web_md-3.8.0")?;
let doc = nlp.process("Alice works at Microsoft in New York.")?;
for (i, token) in doc.tokens().iter().enumerate() {
    println!("{} {:?} {:?}", doc.token_text(TokenIndex(i))?, token.lemma, token.dep);
}
for entity in doc.entities().unwrap() {
    println!("{}: {}", entity.label, doc.span(entity.start, entity.end)?.text());
}
# Ok(())
# }
```

`Model` is immutable, reusable, `Send + Sync`; each call owns its processing state.
`pipe` streams documents sequentially with independent per-document computation.
`process_until(text, Stage::Tagger)` runs an ordered prefix; missing annotations
remain `None`. The API does not silently skip unsupported components.

`Doc` owns exact source text. `ByteOffset`, `CodePointOffset` and `TokenIndex` are
distinct, with checked conversion. Whitespace tokens and trailing ASCII spaces
are preserved. Checked `TokenView`/`SpanView` borrow documents safely. Native
snapshot JSON is versioned and validates boundaries; it is not spaCy DocBin.

`vector`/`token_vector` return `None` for missing lexical keys. `span_vector` and
`document_vector` average all tokens, including OOV zero rows. Similarity returns
1 for identical token sequences (including two empty sequences), otherwise cosine,
or 0 when either vector norm is zero, following the pinned reference. No warnings
are emitted for OOV similarity. Static vectors are distinct from `tok2vec` output.

## Acquire and export explicitly

Requirements: Rust 1.85+ (tested toolchain recorded in reports), Python 3.12.5,
`uv`, and `curl`. Python is only for this development/export workflow.

```sh
uv venv --python 3.12.5 .venv
uv pip sync --python .venv/bin/python tools/reference-requirements.lock
.venv/bin/python tools/acquire.py
.venv/bin/python tools/export.py
.venv/bin/python tools/provenance.py
cargo run --release --example analyze -- assets/en_core_web_md-3.8.0 'Alice met Bob.'
```

The acquisition script verifies the official model wheel SHA-256. Export copies
69 F32 tensors plus configuration, tokenizer/lexical resources, ordered actions,
lookup tables, vectors, licensing and source provenance. Weights are separate from
the Cargo package. The loader checks the weight digest, tensor inventories,
shapes, finite values, references and supported configurations before inference.
See [model format](docs/MODEL_FORMAT.md). Model assets remain local and are not
published by these commands.

## Reproduce verification

After acquiring and exporting the model, fetch the Rust dependencies before
running verification commands that use Cargo offline mode:

```sh
cargo fetch --locked
cargo fetch --locked --manifest-path consumer/Cargo.toml
```

Frozen fixtures are checked in. Do not regenerate expected outputs during fixes.
To audit their provenance, generators under `tools/` run the pinned official
reference. `fixtures/README.md` identifies the development, holdout and regression
sets, numerical tolerances and source licenses.

```sh
cargo fmt --check
cargo fmt --manifest-path consumer/Cargo.toml --check
cargo clippy --all-targets -- -D warnings
cargo test --release -- --include-ignored
cargo build --release --offline --manifest-path consumer/Cargo.toml
env -i PATH=/nonexistent consumer/target/release/native-consumer-check "$PWD/assets/en_core_web_md-3.8.0"
cargo package --allow-dirty --offline
```

For all gates plus a byte-identical re-export check, run
`.venv/bin/python tools/verify.py`. The [CI workflow](.github/workflows/ci.yml)
runs the model-dependent tests using the official exported assets.

Model-dependent tests are explicitly marked ignored for ordinary dependency
builds. **The acceptance command and CI use `--include-ignored`** and fail on
missing assets/fixtures. `cargo test` alone is not a parity run. Reports under
`reports/` contain denominators and expected/actual mismatches. The separate
`consumer` Cargo application verifies all pipeline outputs with an empty runtime
PATH; it does not call Python, Node or any service.

Optional secondary WASM reference (never used by runtime/export): download the
[reference HTML](https://raw.githubusercontent.com/maymay-wa/spacy-wasm/refs/heads/main/spacy-rt-demo.html)
to `reference/wasm-demo.html`, then run:

```sh
.venv/bin/python tools/wasm_reference.py
node tools/wasm_reference.cjs > reports/wasm-development.json
.venv/bin/python tools/compare_wasm.py
```

Extraction rejects a changed HTML checksum. Its annotations are diagnostic only;
official Python remains authoritative. No secondary model assets are shipped.

## Limits

Only the exported English medium configuration above is accepted. Matching APIs,
retokenization, other languages, transformers, training, GPU inference, beam
search, preset NER annotations and spaCy binary serialization remain unsupported.
The disabled `senter` is not executed; sentence boundaries come from the parser.
Scalar numerical kernels prioritize fidelity. No performance or accuracy claims
are made. Finite-corpus parity does not prove all-input compatibility or linguistic
correctness. See `docs/COMPATIBILITY.md` for the broader implementation backlog.

MIT project license; translated upstream code and model resources retain their
own notices in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) and `licenses/`.
