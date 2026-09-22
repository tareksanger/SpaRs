# SpaRs

Experimental, standalone Rust NLP library using **official spaCy pretrained weights**, with native Rust tokenization and inference. The Rust crate needs no Python, JavaScript, WASM, subprocess, network client, or build-time model download. Optional [Node.js bindings](https://github.com/tareksanger/SpaRs/blob/main/docs/NODE.md) live in a separate crate. The project is named **SpaRs**, with Cargo package name `spars-nlp` and Rust import name `spars`.

The implemented target is **en_core_web_md 3.8.0**, exported with **spaCy 3.8.14 / Thinc 8.3.13**. It includes tokenization, lexical features, both tok2vec networks, tags, dependencies, attribute rules, lemmas, NER, sentences, English noun chunks, and static vectors. The declared validation suite covers 190 full-pipeline documents / 7,029 tokens, 4,057 tokenizer cases and 634 transition steps, requiring exact agreement on discrete outputs with the official reference. This is a bounded English inference milestone, **not a port of the entire spaCy library**. Start with the [developer guide](https://github.com/tareksanger/SpaRs/blob/main/docs/DEVELOPMENT.md) for tested examples and the [quality process](https://github.com/tareksanger/SpaRs/blob/main/docs/QUALITY.md) for contribution checks. See [compatibility](https://github.com/tareksanger/SpaRs/blob/main/docs/COMPATIBILITY.md), [progress](https://github.com/tareksanger/SpaRs/blob/main/docs/PROGRESS.md), and [validation](https://github.com/tareksanger/SpaRs/blob/main/docs/VALIDATION.md).

## Use from Rust

Add SpaRs to your application's `Cargo.toml`:

```toml
[dependencies]
spars = { package = "spars-nlp", version = "0.1.0" }
```

```rust
use spars::{Model, TokenIndex};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let nlp = Model::load("assets/en_core_web_md-3.8.0")?;
    let doc = nlp.process("Alice works at Microsoft in New York.")?;
    for (i, token) in doc.tokens().iter().enumerate() {
        println!(
            "{} {:?} {:?}",
            doc.token_text(TokenIndex(i))?,
            token.lemma,
            token.dep
        );
    }
    for entity in doc.entities().unwrap() {
        println!(
            "{}: {}",
            entity.label,
            doc.span(entity.start, entity.end)?.text()
        );
    }
    Ok(())
}
```

`Model` is immutable, reusable, `Send + Sync`; each call owns its processing state. `pipe` streams documents sequentially with independent per-document computation. `process_until(text, Stage::Tagger)` runs an ordered prefix; missing annotations remain `None`. The API does not silently skip unsupported components.

`Doc` owns exact source text. `ByteOffset`, `CodePointOffset` and `TokenIndex` are distinct, with checked conversion. Whitespace tokens and trailing ASCII spaces are preserved. Checked `TokenView`/`SpanView` borrow documents safely. Native snapshot JSON is versioned and validates boundaries; it is not spaCy DocBin.

`vector`/`token_vector` return `None` for missing lexical keys. `span_vector` and `document_vector` average all tokens, including OOV zero rows. Similarity returns 1 for identical token sequences (including two empty sequences), otherwise cosine, or 0 when either vector norm is zero, following the pinned reference. No warnings are emitted for OOV similarity. Static vectors are distinct from `tok2vec` output.

## Install a model without Python

Clone the [SpaRs repository](https://github.com/tareksanger/SpaRs) and run these commands from its root. The native installer and examples are maintained in the repository and are not included in the Cargo crate:

```sh
cargo build --locked --release --manifest-path installer/Cargo.toml
model_dir=$(installer/target/release/spars-model install --version 3.8.0 --root target/models)
cargo run --release --example analyze -- "$model_dir" 'Alice visited New York.'
```

Pass the printed installation directory to `Model::load`. Only the pinned model is supported; installation does not select a new version for your application automatically. See [model installation](https://github.com/tareksanger/SpaRs/blob/main/docs/MODEL_INSTALLATION.md) for local archives, verification, and version handling. The Python setup below is for development and reference testing.

## Acquire and export explicitly

Run these development commands from the cloned repository's root. Requirements for full verification: Rust 1.88+, Python 3.12.5, `uv`, `curl`, and Node.js 24 with npm. The standalone Rust library requires Rust 1.85 and needs neither Python nor Node.js. Python is used for reference tooling; Node.js is used for the optional binding and development checks.

```sh
uv venv --python 3.12.5 .venv
uv pip sync --python .venv/bin/python tools/reference-requirements.lock
npm --prefix tools ci --ignore-scripts --no-audit --no-fund
npm --prefix bindings/node ci --ignore-scripts --no-audit --no-fund
.venv/bin/python tools/acquire.py
.venv/bin/python tools/export.py
cargo run --release --example analyze -- assets/en_core_web_md-3.8.0 'Alice met Bob.'
```

The acquisition script verifies the official model wheel SHA-256. Export copies 69 F32 tensors plus configuration, tokenizer/lexical resources, ordered actions, lookup tables, vectors, licensing and source provenance. Weights are separate from the Cargo package. The loader checks the weight digest, tensor inventories, shapes, finite values, references and supported configurations before inference. See [model format](https://github.com/tareksanger/SpaRs/blob/main/docs/MODEL_FORMAT.md). Model assets remain local and are not published by these commands.

## Reproduce verification

Run development and verification commands from a repository checkout; tests, fixtures, and development tools are excluded from the Cargo package. After acquiring and exporting the model, fetch the Rust dependencies before running verification commands that use Cargo offline mode:

```sh
cargo fetch --locked
cargo fetch --locked --manifest-path consumer/Cargo.toml
cargo fetch --locked --manifest-path installer/Cargo.toml
cargo fetch --locked --manifest-path bindings/node/Cargo.toml
```

Frozen fixtures are checked in. Do not regenerate expected outputs during fixes. To audit their provenance, generators under `tools/` run the pinned official reference. `fixtures/README.md` identifies the development, holdout and regression sets, numerical tolerances and source licenses.

```sh
cargo fmt --check
cargo fmt --manifest-path consumer/Cargo.toml --check
cargo clippy --all-targets -- -D warnings
cargo test --release -- --include-ignored
cargo build --release --offline --manifest-path consumer/Cargo.toml
env -i PATH= consumer/target/release/native-consumer-check assets/en_core_web_md-3.8.0
cargo package --allow-dirty --offline
```

For all gates plus a byte-identical re-export check, run `.venv/bin/python tools/verify.py`. The [CI workflow](https://github.com/tareksanger/SpaRs/blob/main/.github/workflows/ci.yml) runs the model-dependent tests using the official exported assets.

Model-dependent tests are explicitly marked ignored for ordinary dependency builds. **The acceptance command and CI use `--include-ignored`** and fail on missing assets/fixtures. `cargo test` alone is not a parity run. Generated reports under ignored `target/reports/` contain denominators and expected/actual mismatches. In GitHub Actions, open a workflow run and download its `fidelity-reports` artifact for the relevant operating system. Reports are generated for each run and are not stored in Git. The separate `consumer` Cargo application verifies all pipeline outputs with an empty runtime PATH; it does not call Python, Node or any service.

## Limits

Only the exported English medium configuration above is accepted. Advanced matcher options, PhraseMatcher, retokenization, other languages, transformers, training, GPU inference, beam search, preset NER annotations and spaCy binary serialization remain unsupported. The disabled `senter` is not executed; sentence boundaries come from the parser. Native CPU matrix kernels preserve the declared fidelity limits. See [performance](https://github.com/tareksanger/SpaRs/blob/main/docs/PERFORMANCE.md) for commands to measure loading time, throughput, and memory use. No linguistic-accuracy claim is made. Finite-corpus parity does not prove all-input compatibility or linguistic correctness. See `docs/COMPATIBILITY.md` for the broader implementation backlog.

The Cargo crate includes the MIT project license and spaCy/Thinc notices for translated code. Separate model distributions include their model and resource notices. See [third-party attribution](THIRD_PARTY_NOTICES.md).
