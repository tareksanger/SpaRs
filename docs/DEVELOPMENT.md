# Develop with SpaRs

SpaRs turns text into tokens (words and punctuation), identifies names and places, and describes how words relate to each other. The supported model is `en_core_web_md` 3.8.0. It runs inside Rust; Python is needed only to prepare the official model and reference tests.

## Set up the project

Install Rust, `uv`, `curl`, and Node.js 24 with npm, then run the acquisition and export commands in the [README](../README.md#acquire-and-export-explicitly) from the repository root. Fetch both sets of Rust dependencies before running the offline checks:

```sh
cargo fetch --locked
cargo fetch --locked --manifest-path consumer/Cargo.toml
```

The Rust examples below run directly from this Markdown file during verification. Run them on their own with:

```sh
.venv/bin/python tools/check_docs.py
```

## Read tokens and named entities

Load the model once and reuse it. A document owns its original text and annotations. A named entity is a span of text that the model labels, such as a person or place. A span's start is included and its end is excluded.

```rust
use spars::{Model, TokenIndex};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = Model::load("assets/en_core_web_md-3.8.0")?;
    let doc = model.process("Alice works in London.")?;
    assert_eq!(doc.token(TokenIndex(0))?.text(), "Alice");
    let places: Vec<_> = doc
        .entities()
        .unwrap()
        .iter()
        .filter(|entity| entity.label == "GPE")
        .map(|entity| doc.span(entity.start, entity.end).unwrap().text())
        .collect();
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
    let model = Model::load("assets/en_core_web_md-3.8.0")?;
    let doc = model.tokenize("😀 café")?;
    assert_eq!(doc.byte_offset(CodePointOffset(1))?, ByteOffset(4));
    assert!(doc.codepoint_offset(ByteOffset(1)).is_err());
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
    let model = Model::load("assets/en_core_web_md-3.8.0")?;
    let tagged = model.process_until("Birds fly.", Stage::Tagger)?;
    assert!(tagged.tokens()[0].tag.is_some());
    assert!(tagged.entities().is_none());
    let restored = Doc::from_json(&tagged.to_json()?)?;
    assert_eq!(restored, tagged);
    assert_eq!(model.process("")?.entities(), Some([].as_slice()));
    Ok(())
}
```

The saved JSON is SpaRs' own format. It cannot be loaded as a spaCy `DocBin` file.

## Process several documents

`pipe` processes inputs in order. It currently runs sequentially, so it does not promise a speedup over individual calls. Each returned document owns its results.

```rust
use spars::Model;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = Model::load("assets/en_core_web_md-3.8.0")?;
    let texts = ["A dog barked.", "A cat slept."];
    let docs = model.pipe(texts).collect::<spars::Result<Vec<_>>>()?;
    assert_eq!(docs.len(), 2);
    for (doc, text) in docs.iter().zip(texts) {
        assert_eq!(doc, &model.process(text)?);
    }
    assert!(model.vector("dog").is_some());
    assert_eq!(model.similarity(&docs[0], &docs[0]), 1.0);
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
    let model = Model::load("assets/en_core_web_md-3.8.0")?;
    let doc = model.process("Alice works in London.")?;
    let works = doc.token(TokenIndex(1))?;
    assert_eq!(works.children()?.map(|t| t.text()).collect::<Vec<_>>(), ["Alice", "in", "."]);
    assert_eq!(doc.token(TokenIndex(3))?.ancestors()?.map(|t| t.text()).collect::<Vec<_>>(), ["in", "works"]);
    assert_eq!(works.subtree()?.map(|t| t.text()).collect::<Vec<_>>(), ["Alice", "works", "in", "London", "."]);
    assert_eq!(works.sentence()?.text(), "Alice works in London.");
    assert_eq!(doc.sentence_views().unwrap().count(), 1);
    Ok(())
}
```

Children follow token order. Ancestors start with the parent and end at the root; a root has no ancestors. Subtree order matches spaCy: left child subtrees, the token itself, then right child subtrees. With crossing dependencies this may differ from text order, so a subtree is an iterator rather than a contiguous span.

The first dependency query validates all heads in the document and builds a shared index in linear time and memory. An incomplete or cyclic dependency graph anywhere in the document rejects dependency queries. Later child queries visit only the immediate children, ancestor queries follow only the parent chain, and subtree queries visit only that subtree without recursion. Sentence lookup uses the stored sentence spans independently of dependency annotations. Token relationship and sentence queries return errors for missing annotations; `sentence_views()` returns `None` when sentence boundaries are unavailable. A computed empty result is an empty iterator. The cache does not change document equality or saved JSON.

For reusable patterns over these relationships, see [dependency matching](DEPENDENCY_MATCHER.md).
