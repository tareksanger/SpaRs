# Develop with SpaRs

SpaRs turns text into tokens (words and punctuation), identifies names and places, and describes how words relate to each other. The supported models are `en_core_web_sm`, `en_core_web_md`, and `en_core_web_lg` 3.8.0. Examples default to `en_core_web_md`. It runs inside Rust; The [native installer](MODEL_INSTALLATION.md) prepares the supported model without Python. Python is used for development exports and reference tests.

## Set up the project

Install Rust 1.88 or newer for the Node binding, `uv`, `curl`, and Node.js 24 with npm, then run the acquisition and export commands in the [README](../README.md#acquire-and-export-explicitly) from the repository root. The standalone Rust library still requires Rust 1.85. Fetch all Rust dependencies before running the offline checks:

```sh
cargo fetch --locked
cargo fetch --locked --manifest-path consumer/Cargo.toml
npm --prefix bindings/node run build
```

The Rust examples below and the TypeScript examples in the [Node guide](NODE.md) run directly from Markdown during verification. Run them on their own after building the binding with:

```sh
.venv/bin/python tools/check_docs.py
```

## Build the documentation site

The guides in `docs/` are also published as a website with [VitePress](https://vitepress.dev). The site's settings are in `tools/site/.vitepress/config.mjs` and its dependencies are in `tools/package.json`. After `npm --prefix tools ci --ignore-scripts --no-audit --no-fund`, build and view it from the repository root:

```sh
tools/node_modules/.bin/vitepress build tools/site
node tools/site/check-site.mjs
tools/node_modules/.bin/vitepress preview tools/site
```

The build writes the site to the ignored `target/site` folder. It also writes `llms.txt`, a list of every page, and `llms-full.txt`, all pages in one file, for language models to read. The build fails on a link to a guide that does not exist, and when the `sidebar` list in the settings file and the guides in `docs/` disagree, so add each new guide to that list. `tools/site/check-site.mjs` then checks the finished site: every link between pages reaches a file, no link leaves the site without a full address, the code tabs are titled, and the files for language models are present.

A link from a guide to a repository file that is not a guide, such as `../README.md` or a schema in `docs/`, opens that file on GitHub.

Inside a `::: code-group` block, a `rust` example and a `typescript` example are shown as two tabs titled "Rust" and "Node.js". Keep the fences bare, as for any runnable example, so `tools/check_docs.py` still runs both.

The published site holds one copy of the guides per release, chosen from a version menu. `node tools/site/build-versions.mjs` builds and checks them all: it unpacks each release tag that contains that builder, builds it with the site settings and tools the release shipped with, and writes it under its tag, such as `target/site/v1.2.3/`. The latest release is also written to the root of `target/site`, and the current checkout is written under `next`. Until a release contains the builder, the root holds the current checkout and there is no version menu. To build one copy by hand, set `SPARS_SITE_VERSION` to `next` or a release tag before the build and check commands above.

The [documentation workflow](../.github/workflows/docs.yml) runs the builder for pull requests that change the site, and publishes the result to GitHub Pages on a push to the main branch and when a release tag is created. Publishing needs the repository's Pages source set to GitHub Actions.

## Cargo workspace

The virtual workspace at the repository root contains the library in `crates/spars/`, the installer in `crates/spars-model/`, and the Node binding in `bindings/node/`. They share the root `Cargo.lock` and `target/` build directory. Run these commands from the repository root:

```sh
cargo update
cargo check --locked --workspace
cargo fmt --all --check
```

`cargo update` updates all workspace dependencies within the versions allowed by their manifests. Review and commit the root lockfile after updating. Use `cargo build --release -p spars-model` to build the installer; its binaries are in `target/release/`. The root manifest sets `default-members = ["crates/spars"]`, whose package name is `spars-nlp`. Root `cargo build`, `cargo check`, `cargo test`, `cargo run`, and `cargo publish` commands select that package by default; use `-p` to select another member, or `--workspace` with commands that support it.

The `consumer/` project deliberately remains outside the workspace to check SpaRs as an external dependency. Update its separate lockfile with `cargo update --manifest-path consumer/Cargo.toml`. Full acceptance checks both projects, builds the Node addon, and runs its JavaScript tests.

The maintainer command `cargo release` prepares the release PR using GitHub CLI; `cargo release --dry-run` previews it. It is implemented in `tools/release/` and shares the workspace lockfile. See [release preparation](RELEASING.md) for prerequisites and the steps to create a release and publish to crates.io.

## Read tokens and named entities

Load the model once and reuse it. A document owns its original text and annotations. A named entity is a span of text that the model labels, such as a person or place. A span's start is included and its end is excluded.

```rust
use spars::{Model, TokenIndex};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load the model from its folder. Loading is the slow part, so do it once
    // and reuse the model for every text.
    let nlp = Model::load("assets/en_core_web_md-3.8.0")?;

    // Read the sentence. This splits it into tokens (words and punctuation marks)
    // and labels them. The tokens are: Alice(0) works(1) in(2) London(3) .(4)
    let doc = nlp.process("Alice works in London.")?;

    // Tokens are numbered from 0, so token 0 is the first word.
    assert_eq!(doc.token(TokenIndex(0))?.text(), "Alice");

    // An entity is a piece of text the model labels as a name, such as a person
    // or a place. Keep only the entities labelled "GPE" (countries and cities),
    // then turn each one back into its text.
    let places: Vec<_> = doc
        .entities()
        .unwrap()
        .iter()
        .filter(|entity| entity.label == "GPE")
        .map(|entity| doc.span(entity.start, entity.end).unwrap().text())
        .collect();

    // The model found one place in the sentence: London.
    assert_eq!(places, ["London"]);
    Ok(())
}
```

`GPE` is the model's label for a country, city, or similar political area. These labels are predictions, so they can be linguistically wrong even when SpaRs matches spaCy exactly.

## Work with Unicode offsets

A byte offset and a character offset are different. The emoji below occupies four UTF-8 bytes but one Unicode code point. `TokenIndex` counts tokens instead. Use the checked conversion methods rather than treating these numbers as interchangeable.

```rust
use spars::{ByteOffset, CodePointOffset, Model};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load the model once.
    let nlp = Model::load("assets/en_core_web_md-3.8.0")?;

    // Only split the text into tokens (words and symbols). This skips the later
    // steps, such as finding names, because this example only needs positions.
    let doc = nlp.tokenize("😀 café")?;

    // A position in text can be counted in two ways. A code-point offset counts
    // Unicode characters. A byte offset counts the bytes that store them.
    // The emoji is 1 code point but 4 bytes, so the space after it is at
    // code point 1 and at byte 4. Both counts start at 0.
    assert_eq!(doc.byte_offset(CodePointOffset(1))?, ByteOffset(4));

    // Byte 1 is in the middle of the emoji, not at the start of a character,
    // so the conversion returns an error instead of a wrong answer.
    assert!(doc.codepoint_offset(ByteOffset(1)).is_err());

    // The document keeps the original text exactly as it was given.
    assert_eq!(doc.text(), "😀 café");
    Ok(())
}
```

A code point is not always a whole visible character: accents and joined emoji can contain several code points.

## Run part of the pipeline and save a document

A pipeline is the ordered set of processing steps. For example, the tagger assigns grammatical tags. Stopping at the tagger leaves entities unavailable, represented by `None`. Running the whole pipeline on empty text produces a computed empty entity list instead.

```rust
use spars::{Doc, Model, Stage};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load the model once.
    let nlp = Model::load("assets/en_core_web_md-3.8.0")?;

    // A stage is one step of the pipeline. Run the steps up to and including
    // the tagger, then stop. The tagger gives each token a grammatical tag,
    // a short code for the kind of word it is, such as a plural noun.
    let tagged = nlp.process_until("Birds fly.", Stage::Tagger)?;

    // The tagger ran, so the first token ("Birds") has a tag.
    assert!(tagged.tokens()[0].tag.is_some());

    // The step that finds entities (labelled names) did not run, so the answer
    // is `None`, meaning "not available". That is different from "none found".
    assert!(tagged.entities().is_none());

    // Save the document as JSON text, then load it back. The saved copy is a
    // snapshot: a record of the document exactly as it is at this moment.
    let restored = Doc::from_json(&tagged.to_json()?)?;

    // The loaded copy equals the original, so nothing was lost by saving.
    assert_eq!(restored, tagged);

    // Run the whole pipeline on empty text. This time the entity step did run
    // and found nothing, so the answer is an empty list rather than `None`.
    assert_eq!(nlp.process("")?.entities(), Some([].as_slice()));
    Ok(())
}
```

The saved JSON is SpaRs' own format. It cannot be loaded as a spaCy `DocBin` file.

## Process several documents

`pipe` processes inputs in order. It currently runs sequentially, so it does not promise a speedup over individual calls. Each returned document owns its results.

```rust
use spars::Model;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load the model once.
    let nlp = Model::load("assets/en_core_web_md-3.8.0")?;

    // Two separate texts to read.
    let texts = ["A dog barked.", "A cat slept."];

    // `pipe` reads the texts one after another, in the order given. Collecting
    // the results stops at the first error, or gives one document per text.
    let docs = nlp.pipe(texts).collect::<spars::Result<Vec<_>>>()?;

    // Two texts went in, so two documents come out.
    assert_eq!(docs.len(), 2);

    // Each document is the same as the one made by reading its text alone,
    // so `pipe` is a shortcut and does not change the results.
    for (doc, text) in docs.iter().zip(texts) {
        assert_eq!(doc, &nlp.process(text)?);
    }

    // A vector is a list of numbers that stands for a word's meaning. Words
    // with related meanings get vectors that are close together. This model
    // stores a vector for the word "dog".
    assert!(nlp.vector("dog").is_some());

    // Similarity is a score for how alike two documents are, worked out from
    // their vectors. A document compared with itself scores exactly 1.0.
    assert_eq!(nlp.similarity(&docs[0], &docs[0]), 1.0);
    Ok(())
}
```

## Add or fix a feature

Follow the [quality process](QUALITY.md). Start with a small behavior you can compare against official spaCy, add a test that exposes the missing behavior, implement it, and run the acceptance command. Keep expected results fixed while fixing code. Update the [compatibility inventory](COMPATIBILITY.md) with the supported inputs and evidence.

For example, `Wait—didn't you say that?` exercises how the tokenizer treats a contraction after a dash. A useful regression checks every token and its offset, then checks the resulting annotations against spaCy. Checking only the number of tokens would miss several incorrect outputs.

## Inspect grammatical relationships

A token's children are the words directly attached to it. Its ancestors follow the chain from its parent to the sentence root. Its subtree includes the token and all words attached below it. Load a parsed document before requesting these relationships.

```rust
use spars::{Model, TokenIndex};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load the model once.
    let nlp = Model::load("assets/en_core_web_md-3.8.0")?;

    // Read the sentence with the whole pipeline. This includes the parser, which
    // works out the dependencies: links that attach each word to the word it
    // belongs with (its parent). One word, the root, is the top of the sentence;
    // SpaRs records the root as its own parent.
    // The tokens are: Alice(0) works(1) in(2) London(3) .(4)
    let doc = nlp.process("Alice works in London.")?;

    // Token 1 is the verb "works". It is the root of this sentence.
    let works = doc.token(TokenIndex(1))?;

    // Children are the words attached directly to a token. "London" is missing
    // because it is attached to "in", not to "works".
    assert_eq!(works.children()?.map(|t| t.text()).collect::<Vec<_>>(), ["Alice", "in", "."]);

    // Ancestors are found by walking upward from a token: its parent, then the
    // parent's parent, up to the root. From "London" (token 3) that is "in",
    // then "works".
    assert_eq!(doc.token(TokenIndex(3))?.ancestors()?.map(|t| t.text()).collect::<Vec<_>>(), ["in", "works"]);

    // A subtree is a token plus every word attached below it. "works" is the
    // root, so its subtree is the whole sentence.
    assert_eq!(works.subtree()?.map(|t| t.text()).collect::<Vec<_>>(), ["Alice", "works", "in", "London", "."]);

    // Ask which sentence a token belongs to. The model also marks where each
    // sentence starts and ends.
    assert_eq!(works.sentence()?.text(), "Alice works in London.");

    // The text holds exactly one sentence.
    assert_eq!(doc.sentence_views().unwrap().count(), 1);
    Ok(())
}
```

Children follow token order. Ancestors start with the parent and end at the root; a root has no ancestors. Subtree order matches spaCy: left child subtrees, the token itself, then right child subtrees. With crossing dependencies this may differ from text order, so a subtree is an iterator rather than a contiguous span.

The first dependency query validates all heads in the document and builds a shared index in linear time and memory. An incomplete or cyclic dependency graph anywhere in the document rejects dependency queries. Later child queries visit only the immediate children, ancestor queries follow only the parent chain, and subtree queries visit only that subtree without recursion. Sentence lookup uses the stored sentence spans independently of dependency annotations. Token relationship and sentence queries return errors for missing annotations; `sentence_views()` returns `None` when sentence boundaries are unavailable. A computed empty result is an empty iterator. The cache does not change document equality or saved JSON.

For reusable patterns over these relationships, see [dependency matching](DEPENDENCY_MATCHER.md).

For contiguous sequences and repetition, see [Token Matcher](TOKEN_MATCHER.md).

## Select a model size

After the README's `--all` acquisition and export commands, the same loader accepts all three pinned English sizes. The small model provides contextual document vectors; the other sizes provide static averages. This example runs during documentation acceptance.

```rust
use spars::Model;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Try each model size in turn: small (sm), medium (md) and large (lg).
    // The number beside each name is how long its document vector should be.
    for (name, width) in [("en_core_web_sm", 96), ("en_core_web_md", 300), ("en_core_web_lg", 300)] {
        // Load this size of model from its folder. The same call works for all three.
        let nlp = Model::load(format!("assets/{name}-3.8.0"))?;

        // Read a sentence with the whole pipeline.
        let doc = nlp.process("Alice visited London.")?;

        // Every size looks for entities (labelled names such as people and
        // places), so the entity list is available.
        assert!(doc.entities().is_some());

        // A document vector is a list of numbers that stands for the meaning of
        // the whole text. The small model gives 96 numbers; the others give 300.
        assert_eq!(nlp.document_vector(&doc).len(), width);
    }

    // Reaching this line means all three sizes loaded and passed the checks.
    Ok(())
}
```

This does not establish support for other languages, transformer models, or custom components. Capability declarations and configuration are validated before inference; additional implementations and reference suites are required for those families.

## Resolve an installed model by name

Set `SPARS_MODEL_DIR` and explicitly download the model first, as described in [installation](MODEL_INSTALLATION.md). The guide checker prepares the small model in its own store before executing this example. A missing installation returns an error with a download hint; loading never starts a download. Use `./directory` or an absolute path to load an export directly instead of resolving a bare name.

```rust
use spars::Model;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load the model by its name instead of a folder path. The model must already
    // be downloaded into the folder named by the `SPARS_MODEL_DIR` setting;
    // loading never downloads anything, and returns an error if it is missing.
    let nlp = Model::load("en_core_web_sm")?;

    // Read a sentence with the whole pipeline.
    let doc = nlp.process("Alice visits London.")?;

    // The pipeline looked for entities (labelled names such as people and
    // places), so the entity list is available.
    assert!(doc.entities().is_some());

    // A document vector is a list of numbers that stands for the meaning of the
    // whole text. The small model's vector has 96 numbers, which shows that the
    // name resolved to the small model.
    assert_eq!(nlp.document_vector(&doc).len(), 96);
    Ok(())
}
```

## Handle errors

Every SpaRs function that can fail returns a `Result`. The error type is `spars::Error`, and each kind of problem has its own variant (one named case of the error type), so a program can tell them apart and decide what to do. This example causes three different errors on purpose. It needs the `en_core_web_sm` model from the [quick setup](index.md#quick-setup).

```rust
use spars::{Error, Model, Stage, TokenIndex};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Asking for a model that is not installed gives an I/O error.
    assert!(matches!(Model::load("en_core_web_xx"), Err(Error::Io(_))));

    // Load a model that is installed.
    let nlp = Model::load("en_core_web_sm")?;

    // This document has 4 tokens: Dogs(0) chase(1) cats(2) .(3)
    let doc = nlp.process("Dogs chase cats.")?;

    // Asking for token 99 is out of range, which gives `Error::Bounds`.
    assert!(matches!(doc.token_text(TokenIndex(99)), Err(Error::Bounds)));

    // Stop after the tokenizer, so the document has tokens but no sentences.
    let tokens_only = nlp.process_until("Dogs chase cats.", Stage::Tokenizer)?;

    // Asking which sentence a token is in then fails, and the error names the
    // missing annotation.
    let first = tokens_only.token(TokenIndex(0))?;
    assert!(matches!(first.sentence(), Err(Error::MissingAnnotation("sentences"))));

    // Printing an error gives a short message for a person to read.
    let message = doc.token_text(TokenIndex(99)).unwrap_err().to_string();
    assert_eq!(message, "invalid span or offset");
    Ok(())
}
```

These are all the variants:

| Variant | When you see it |
|---|---|
| `Io` | A file or folder could not be read. A model that is not installed is reported this way; see [troubleshooting](TROUBLESHOOTING.md). |
| `Json` | Saved JSON, such as a document snapshot or a model's manifest, could not be read. |
| `Model` | A model's files, a model store setting, or a saved document's contents are not valid. `ModelStore` also returns it for a name that is not a valid model name. |
| `Unsupported` | The model asks for something SpaRs does not support, or a requested stage is not in the model's pipeline. Also a saved document in a format SpaRs cannot read, or annotations that a matcher or an entity update cannot use. |
| `Pattern` | A matcher or ruler pattern is not valid. |
| `Bounds` | A token index, span or offset is outside the document. |
| `MissingAnnotation` | The document lacks an annotation the call needs, usually because the stage that produces it did not run. The value names the annotation. Calls that read a whole list, such as `Doc::sentences`, return `None` instead; see [a value is missing](TROUBLESHOOTING.md#a-value-is-missing). |
| `Dependency` | The document's head and dependency data do not form a valid tree. |
| `InvalidSentence` | No sentence contains the given token. |
| `Regex` | A regular expression stored in the model's tokenizer or word rules could not be compiled or run. You should not see this with an official model. |

## Look up facts about a word

A **lexeme** is a word looked at by itself, without a sentence around it. `Model::lexeme` answers questions about a word on its own, using its spelling and the model's word lists: is it all letters, does it look like a number, is it a very common word? This example needs the `en_core_web_sm` model from the [quick setup](index.md#quick-setup).

```rust
use spars::Model;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load the model once and reuse it for every text.
    let nlp = Model::load("en_core_web_sm")?;

    // Look up "Apple". The norm is a standard spelling used to treat variants of
    // a word alike, here lower case. The shape replaces capital letters with X,
    // small letters with x and digits with d, which shows the pattern of a word
    // without its letters. A run of the same symbol is cut off after four.
    let apple = nlp.lexeme("Apple")?;
    assert_eq!(apple.norm, "apple");
    assert_eq!(apple.shape, "Xxxxx");

    // The prefix is the first character and the suffix is the last three.
    assert_eq!(apple.prefix, "A");
    assert_eq!(apple.suffix, "ple");

    // `is_alpha` means every character is a letter. `is_title` means a capital
    // letter followed by small letters.
    assert!(apple.is_alpha && apple.is_title);

    // A stop word is a very common word, such as "the", that often says little
    // about what a text is about.
    assert!(nlp.lexeme("the")?.is_stop);

    // `like_num` is true for words that look like numbers, written in digits or
    // in letters.
    assert!(nlp.lexeme("3.50")?.like_num);
    assert!(nlp.lexeme("ten")?.like_num);

    // `like_email` is true for text shaped like an email address.
    assert!(nlp.lexeme("hello@example.com")?.like_email);
    Ok(())
}
```

## Read token and span vectors

A **vector** is a list of numbers that stands for meaning; [choose a model](MODELS.md#what-word-vectors-are) explains the idea. `Model::token_vector` gives the vector of one token, `Model::span_vector` the average over a span (a run of neighbouring tokens), and `Model::document_vector` the average over the whole document. `Model::span_similarity` and `Model::similarity` compare two spans or two documents and return a score: 1 for identical text and lower for less similar text. The score can be 0 or negative. This example needs the `en_core_web_sm` model from the [quick setup](index.md#quick-setup).

```rust
use spars::{Model, TokenIndex};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load the small model. It works out each token's vector from the words
    // around it, and every vector has 96 numbers.
    let nlp = Model::load("en_core_web_sm")?;

    // The tokens are:
    // Dogs(0) chase(1) cats(2) .(3) Puppies(4) chase(5) kittens(6) .(7)
    let doc = nlp.process("Dogs chase cats. Puppies chase kittens.")?;

    // The vector of one token. The result is optional: it is `None` when the
    // document has no vectors, for example after stopping at the tokenizer.
    let dogs = nlp.token_vector(doc.token(TokenIndex(0))?).unwrap();
    assert_eq!(dogs.len(), 96);

    // The vector of a span is the average of its tokens' vectors. The span from
    // 0 to 3 is "Dogs chase cats": the end index is not included.
    let span_vector = nlp.span_vector(&doc, TokenIndex(0), TokenIndex(3))?;
    assert_eq!(span_vector.len(), 96);

    // Compare two spans with different words. The score is below 1.
    let first = doc.span(TokenIndex(0), TokenIndex(3))?;
    let second = doc.span(TokenIndex(4), TokenIndex(7))?;
    assert_eq!(second.text(), "Puppies chase kittens");
    let score = nlp.span_similarity(first, second);
    assert!(score > 0.0 && score < 1.0);

    // A span compared with itself scores exactly 1.
    assert_eq!(nlp.span_similarity(first, first), 1.0);

    // The small model stores no vector for a word looked up by itself.
    assert!(nlp.vector("dog").is_none());
    Ok(())
}
```

## Work with a model store

A **model store** is the folder where downloaded models live. `Model::load("en_core_web_sm")` uses the default store. `ModelStore` lets a program choose the folder itself, or check whether a model is installed before loading it. [Troubleshooting](TROUBLESHOOTING.md#how-spars-finds-a-model) explains how the default store is chosen. This example needs the `en_core_web_sm` model from the [quick setup](index.md#quick-setup).

```rust
use spars::{Error, ModelStore};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Find the default store: the folder named by the `SPARS_MODEL_DIR` setting,
    // or a `spars/models` folder inside your user's cache folder when it is not set.
    let store = ModelStore::discover()?;

    // Ask where a model is installed. This reads a small record and does not
    // load the model, so it is fast.
    let folder = store.resolve("en_core_web_sm")?;
    assert!(folder.starts_with(store.root()));

    // A model that is not installed gives an I/O error, as `Model::load` does.
    assert!(matches!(store.resolve("en_core_web_xx"), Err(Error::Io(_))));

    // Load the model from this store.
    let nlp = store.load("en_core_web_sm")?;
    assert_eq!(nlp.process("Hello there.")?.tokens().len(), 3);

    // To use another folder, name it. Creating the store reads nothing; the
    // folder is only looked at when you resolve or load a model. This folder
    // holds no models, so the model is reported as not installed.
    let other = ModelStore::new("target/another-store");
    assert!(matches!(other.resolve("en_core_web_sm"), Err(Error::Io(_))));
    Ok(())
}
```

## Estimate a document's memory

`Doc::estimated_heap_bytes` gives the approximate memory a document holds, in bytes. It is a lower bound, meaning the real figure is at least this much: it leaves out the `Doc` value itself, an index built the first time you walk the grammar tree, and the memory allocator's own overhead. It is useful for comparing documents or for limiting how many a program keeps at once. This example needs the `en_core_web_sm` model from the [quick setup](index.md#quick-setup).

```rust
use spars::Model;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load the model once and reuse it for every text.
    let nlp = Model::load("en_core_web_sm")?;

    // Read a short text and a longer one.
    let short = nlp.process("Hi.")?;
    let long = nlp.process("Dogs chase cats. Puppies chase kittens.")?;

    // A longer document holds more tokens, so it uses more memory.
    assert!(short.estimated_heap_bytes() > 0);
    assert!(long.estimated_heap_bytes() > short.estimated_heap_bytes());
    Ok(())
}
```

## API behavior notes

`Model` is immutable, reusable, `Send + Sync`; each call owns its processing state. `pipe` streams documents sequentially with independent per-document computation. `process_until(text, Stage::Tagger)` runs an ordered prefix; missing annotations remain `None`. The API does not silently skip unsupported components.

`Doc` owns exact source text. `ByteOffset`, `CodePointOffset` and `TokenIndex` are distinct, with checked conversion. Whitespace tokens and trailing ASCII spaces are preserved. Checked `TokenView`/`SpanView` borrow documents safely. Native snapshot JSON is versioned and validates boundaries and closed values (entity IOB tags, universal POS tags, non-empty entity labels, empty sentence labels and `NP` noun-chunk labels) and entities that agree with the token IOB tags; it is not spaCy DocBin.

`vector` returns static lexical vectors or `None` for missing keys. For `sm`, `token_vector` uses contextual rows produced by the shared encoder; it returns `None` before encoding. `span_vector` and `document_vector` average all tokens, including OOV zero rows for static models. Empty `sm` documents/spans return zero-length vectors. Contextual rows survive native v2 document snapshots. Similarity returns 1 for identical token sequences (including two empty sequences), otherwise cosine, or 0 when either vector norm is zero, following the pinned reference. No warnings are emitted for OOV similarity. Static vectors are distinct from `tok2vec` output.

`Doc::set_entities` replaces entity annotation like spaCy's `Doc.set_ents`, keeping token IOB tags, entity types and the entity list consistent and rejecting invalid updates without changing the document; `Doc::with_entities` returns an updated copy instead. See [editing entities](ENTITIES.md).

`EntityRuler` adds entities from token patterns and phrases like spaCy's `entity_ruler`, after the recognizer or without it, with spaCy's priority, overwrite and entity-ID behavior and a few [documented differences](ENTITY_RULER.md#differences-from-spacy). See [adding entities with rules](ENTITY_RULER.md); Node exposes it as `EntityRuler`.

Rust documents store named [span groups](SPAN_GROUPS.md) like spaCy's `Doc.spans`: ordered lists of spans that may overlap, with names and JSON attributes, kept in snapshots; Node exposes them through `withSpanGroup`, `withoutSpanGroup` and `toObject().spanGroups`.

`SpanRuler` adds span groups, and optionally entities, from token patterns and phrases like spaCy's `span_ruler`, with spaCy's filters, overwrite behavior and match order. See [adding spans with rules](SPAN_RULER.md); Node provides it as [`SpanRuler`](NODE.md#add-spans-with-rules).

`PhraseMatcher` matches reusable phrase patterns by exact text (`ORTH`/`TEXT`), pinned Unicode lowercase (`LOWER`), token norm (`NORM`), linguistic annotations (`LEMMA`, `POS`, `TAG`, `DEP` and `MORPH`), lexical flags such as `IS_ALPHA`, or `LENGTH`, including ordered overlaps and registration/removal. It accepts existing documents without inference or downloads. See the [PhraseMatcher guide](PHRASE_MATCHER.md). The [Node API](NODE.md) exposes all three native matchers, the EntityRuler and SpanRuler, retained document/token/span views, entity editing, span groups, traversal, vectors and similarity, lexical queries, snapshots, and offline model-store management.
