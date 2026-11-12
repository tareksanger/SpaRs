# SpaRs

[![crates.io](https://img.shields.io/crates/v/spars-nlp.svg)](https://crates.io/crates/spars-nlp) [![docs.rs](https://img.shields.io/docsrs/spars-nlp)](https://docs.rs/spars-nlp) [![npm](https://img.shields.io/npm/v/@spars/node.svg)](https://www.npmjs.com/package/@spars/node) [![CI](https://github.com/tareksanger/SpaRs/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/tareksanger/SpaRs/actions/workflows/ci.yml) [![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/tareksanger/SpaRs/blob/main/LICENSE) [![MSRV 1.85](https://img.shields.io/badge/MSRV-1.85-orange.svg)](https://github.com/tareksanger/SpaRs/blob/main/crates/spars/Cargo.toml)

**Fast, native natural language processing for Rust and Node.js, using the official [spaCy](https://spacy.io) pretrained models.**

SpaRs reads English text and tells you what is in it: the words, their grammar, the sentences, and the names of people, places and organizations. It runs on your own machine with no Python, no network calls while it works, and no model download at build time.

> **Built on spaCy.** SpaRs loads the pretrained models published by the [spaCy](https://spacy.io) team at [Explosion](https://explosion.ai), follows spaCy's behavior as its reference, and checks its results against spaCy's. If you work in Python, or need training, other languages or transformer models, use [spaCy](https://github.com/explosion/spaCy) itself. SpaRs is an independent project and is not made or endorsed by Explosion.

> **AI disclosure:** This project is built with AI assistance.

## Features

- **Tokens and grammar:** tokenization, lemmas, part-of-speech tags, dependency parsing, sentences and English noun chunks.
- **Named entities:** people, places, organizations and more, with [entity editing](https://github.com/tareksanger/SpaRs/blob/main/docs/ENTITIES.md).
- **Rule-based matching:** [token](https://github.com/tareksanger/SpaRs/blob/main/docs/TOKEN_MATCHER.md), [phrase](https://github.com/tareksanger/SpaRs/blob/main/docs/PHRASE_MATCHER.md) and [dependency](https://github.com/tareksanger/SpaRs/blob/main/docs/DEPENDENCY_MATCHER.md) matchers, plus an [entity ruler](https://github.com/tareksanger/SpaRs/blob/main/docs/ENTITY_RULER.md) and a [span ruler](https://github.com/tareksanger/SpaRs/blob/main/docs/SPAN_RULER.md).
- **Vectors and similarity:** word, span and document vectors, and similarity scores.
- **Two languages, one engine:** a Rust crate and a typed [Node.js package](https://github.com/tareksanger/SpaRs/blob/main/docs/NODE.md) that share the same native code and models.
- **Checked against spaCy:** discrete outputs must agree exactly with the official reference on the project's [validation suites](https://github.com/tareksanger/SpaRs/blob/main/docs/VALIDATION.md).

## Quick start

Setup has two steps: install the library, then download a model. A model is a folder of files holding the learned numbers that SpaRs needs before it can read text.

### Rust

Add the library. The package is named `spars-nlp` and is imported as `spars`:

```sh
cargo add spars-nlp --rename spars
```

Download a model with the `spars` tool, built from a clone of this repository:

```sh
cargo install --path crates/spars-model --bin spars --locked
spars download en_core_web_sm
```

Then read a sentence:

```rust
use spars::{Model, TokenIndex};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load the model once and reuse it for every text.
    let nlp = Model::load("en_core_web_sm")?;

    // Read a sentence. The result is a document: the text split into tokens
    // (words and punctuation marks), with what the model worked out about each one.
    let doc = nlp.process("Alice works at Microsoft in London.")?;

    // Print every token with its lemma (dictionary form) and part of speech.
    for (index, token) in doc.tokens().iter().enumerate() {
        let text = doc.token_text(TokenIndex(index))?;
        println!("{text}\t{:?}\t{:?}", token.lemma, token.pos);
    }

    // Collect the entities: the real-world things named in the text.
    let mut found = Vec::new();
    for entity in doc.entities().unwrap() {
        let text = doc.span(entity.start, entity.end)?.text().to_string();
        found.push(format!("{}: {text}", entity.label));
    }

    // The model found a person, an organization and a place.
    assert_eq!(found, ["PERSON: Alice", "ORG: Microsoft", "GPE: London"]);
    Ok(())
}
```

### Node.js

Use Node.js 22 or newer. Install the package and download a model with the tool that comes with it:

```sh
npm install @spars/node
npx spars download en_core_web_sm
```

Then read a sentence:

```typescript
import assert from 'node:assert/strict';
import { loadModel } from '@spars/node';

// Load the model once and reuse it for every text.
const nlp = await loadModel('en_core_web_sm');

// Read a sentence. The result is a document: the text split into tokens
// (words and punctuation marks), with what the model worked out about each one.
const doc = await nlp.process('Alice works at Microsoft in London.');

// Print every token with its lemma (dictionary form) and part of speech.
for (const token of doc.tokens) {
  console.log(`${token.text}\t${token.lemma}\t${token.pos}`);
}

// Collect the entity labels: the kinds of real-world things named in the text.
assert.ok(doc.entities);
const labels = doc.entities.map(entity => entity.label);

// The model found a person, an organization and a place.
assert.deepEqual(labels, ['PERSON', 'ORG', 'GPE']);
```

## Supported models

SpaRs runs three official English models, version 3.8.0, exported with spaCy 3.8.14 and Thinc 8.3.13.

| Model | Best for |
|---|---|
| `en_core_web_sm` | Trying SpaRs out. It is the smallest download. |
| `en_core_web_md` | Comparing the meaning of words and texts. It includes word vectors. |
| `en_core_web_lg` | The same as `md`, with many more distinct vectors, so fewer words share one. |

Downloading a model is always an explicit step; loading and processing never use the network. See [installing a model](https://github.com/tareksanger/SpaRs/blob/main/docs/MODEL_INSTALLATION.md).

## Documentation

| Guide | What it covers |
|---|---|
| [Developer guide](https://github.com/tareksanger/SpaRs/blob/main/docs/DEVELOPMENT.md) | Rust examples for tokens, entities, offsets, sentences, vectors and partial pipelines |
| [Node.js guide](https://github.com/tareksanger/SpaRs/blob/main/docs/NODE.md) | The whole Node.js API, with examples |
| [Installing a model](https://github.com/tareksanger/SpaRs/blob/main/docs/MODEL_INSTALLATION.md) | Downloading models, where they are stored, and local archives |
| [Compatibility](https://github.com/tareksanger/SpaRs/blob/main/docs/COMPATIBILITY.md) | What is supported and what is not |
| [Differences from spaCy](https://github.com/tareksanger/SpaRs/blob/main/docs/SPACY_DIFFERENCES.md) | Where SpaRs behaves differently on purpose |
| [Validation](https://github.com/tareksanger/SpaRs/blob/main/docs/VALIDATION.md) | How results are checked against spaCy |
| [Performance](https://github.com/tareksanger/SpaRs/blob/main/docs/PERFORMANCE.md) | How to measure loading time, throughput and memory |
| [Rust API reference](https://docs.rs/spars-nlp) | Every type and function in the Rust library |

## Status and limits

SpaRs is experimental. It covers a bounded part of spaCy, not the whole library.

- **Supported:** the three English models above, on CPU.
- **Not supported:** other languages, transformer models, training, GPU inference, retokenization, beam search, and spaCy's binary serialization (DocBin).
- **Planned:** native inference for all official spaCy pretrained pipelines. This is a goal, not current support; see the [model extensibility plan](https://github.com/tareksanger/SpaRs/blob/main/docs/PROGRESS.md#model-extensibility-plan).
- **What agreement with spaCy means:** SpaRs is tested for agreement with spaCy on fixed sets of text. That does not prove agreement on every input, and it makes no claim about linguistic accuracy.

## Development

The Rust library needs Rust 1.85. Working on SpaRs itself also needs Python, `uv` and Node.js for the reference tools and the Node.js binding. The [developer guide](https://github.com/tareksanger/SpaRs/blob/main/docs/DEVELOPMENT.md) and the [quality process](https://github.com/tareksanger/SpaRs/blob/main/docs/QUALITY.md) describe the workflow.

### Acquire and export explicitly

Run these development commands from the cloned repository's root. Requirements for full verification: Rust 1.88+, Python 3.12.5, `uv`, `curl`, and Node.js 24 with npm. The standalone Rust library requires Rust 1.85 and needs neither Python nor Node.js. Python is used for reference tooling; Node.js is used for the optional binding and development checks.

```sh
uv venv --python 3.12.5 .venv
uv pip sync --python .venv/bin/python tools/reference-requirements.lock
npm --prefix tools ci --ignore-scripts --no-audit --no-fund
npm --prefix bindings/node ci --ignore-scripts --no-audit --no-fund
.venv/bin/python tools/acquire.py --all
.venv/bin/python tools/export.py --all
cargo run --release --example analyze -- assets/en_core_web_md-3.8.0 'Alice met Bob.'
```

The acquisition script verifies the official model wheel SHA-256. Export copies 67 (`sm`) or 69 (`md`/`lg`) F32 tensors plus configuration, tokenizer/lexical resources, ordered actions, lookup tables, vectors, licensing and source provenance. Weights are separate from the Cargo package. The loader checks the weight digest, tensor inventories, shapes, finite values, references and supported configurations before inference. See [model format](https://github.com/tareksanger/SpaRs/blob/main/docs/MODEL_FORMAT.md). Model assets remain local and are not published by these commands.

### Reproduce verification

Run development and verification commands from a repository checkout; tests, fixtures, and development tools are excluded from the Cargo package. After acquiring and exporting the model, fetch the Rust dependencies before running verification commands that use Cargo offline mode:

```sh
cargo fetch --locked
cargo fetch --locked --manifest-path consumer/Cargo.toml
```

Frozen fixtures are checked in. Do not regenerate expected outputs during fixes. To audit their provenance, generators under `tools/` run the pinned official reference. `fixtures/README.md` identifies the development, holdout and regression sets, numerical tolerances and source licenses.

```sh
cargo fmt --all --check
cargo fmt --manifest-path consumer/Cargo.toml --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --release -- --include-ignored
cargo build --release --offline --manifest-path consumer/Cargo.toml
env -i PATH= consumer/target/release/native-consumer-check assets/en_core_web_md-3.8.0
cargo package -p spars-nlp --allow-dirty --offline
```

For all gates plus a byte-identical re-export check, run `.venv/bin/python tools/verify.py`. The [CI workflow](https://github.com/tareksanger/SpaRs/blob/main/.github/workflows/ci.yml) runs the model-dependent tests using the official exported assets.

Model-dependent tests are explicitly marked ignored for ordinary dependency builds. **The acceptance command and CI use `--include-ignored`** and fail on missing assets/fixtures. `cargo test` alone is not a parity run. Generated reports under ignored `target/reports/` contain denominators and expected/actual mismatches. In GitHub Actions, open a workflow run and download its `fidelity-reports` artifact for the relevant operating system. Reports are generated for each run and are not stored in Git. The separate `consumer` Cargo application verifies all pipeline outputs with an empty runtime PATH; it does not call Python, Node or any service.

## Contributing

Contributions are welcome. Read [CONTRIBUTING.md](https://github.com/tareksanger/SpaRs/blob/main/CONTRIBUTING.md) before opening a pull request, and report bugs or propose features in the [issue tracker](https://github.com/tareksanger/SpaRs/issues). To report a security problem, follow [SECURITY.md](https://github.com/tareksanger/SpaRs/blob/main/SECURITY.md).

## Acknowledgements

SpaRs depends on the work of the [spaCy](https://spacy.io) and [Thinc](https://thinc.ai) projects by [Explosion](https://explosion.ai). The pretrained models, the pipeline design and the reference behavior are theirs. Parts of SpaRs are translated from spaCy and Thinc source code under their MIT licenses.

## License

SpaRs is released under the [MIT license](https://github.com/tareksanger/SpaRs/blob/main/LICENSE). The Cargo crate includes the MIT project license and spaCy/Thinc notices for translated code. Separate model distributions include their model and resource notices. See [third-party attribution](https://github.com/tareksanger/SpaRs/blob/main/THIRD_PARTY_NOTICES.md).
