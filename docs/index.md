# SpaRs

SpaRs reads English text and tells you what is in it: the words, their grammar, the sentences, and the names of people, places and organizations. It is written in Rust, and you can use it from Rust or from Node.js. It runs on your own computer, with no Python and no network connection while it works.

## Built on spaCy

SpaRs exists because of [spaCy](https://spacy.io), the open-source Python library for natural language processing made by [Explosion](https://explosion.ai). SpaRs loads the official pretrained models that the spaCy team publishes, follows spaCy's behavior as its reference, and checks its results against spaCy's. The models, the design of the pipeline, and the rules SpaRs follows are spaCy's work.

If you work in Python, or you need training, other languages, or transformer models, use spaCy itself:

- [spacy.io](https://spacy.io) is the home of spaCy, with its guides and API reference.
- [explosion/spaCy](https://github.com/explosion/spaCy) is the source code.
- [spaCy models](https://spacy.io/models) lists the pretrained models, including the three that SpaRs runs.

SpaRs is an independent project. It is not made, endorsed or supported by Explosion. spaCy and its models keep their own licenses, which are listed in the [third-party notices](../THIRD_PARTY_NOTICES.md).

## Quick setup

Setup has two steps: install the library, then download a model. A model is a folder of files holding the learned numbers that SpaRs needs before it can read text. Models are downloaded once and stored on your computer; both Rust and Node.js look for them in the same place.

### Rust

Add the library to your project. The package is named `spars-nlp` and is imported as `spars`:

```sh
cargo add spars-nlp --rename spars
```

Then download a model. The download tool is built from a copy of the SpaRs repository, so run these from the repository root:

```sh
cargo install --path crates/spars-model --bin spars --locked
spars download en_core_web_sm
```

### Node.js

Use Node.js 22 or newer. Install the package, then download a model with the tool that comes with it:

```sh
npm install @spars/node
npx spars download en_core_web_sm
```

[Installing a model](MODEL_INSTALLATION.md) explains where models are stored, how to choose the folder, and how to install from a file you already downloaded.

## Getting started

This first program loads the model, reads one sentence, and prints what SpaRs found in it.

::: code-group

```rust
use spars::{Model, TokenIndex};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load the model you downloaded. Do this once and reuse it: loading is the
    // slow part, and one model can read as many texts as you like.
    let nlp = Model::load("en_core_web_sm")?;

    // Read a sentence. The result is a document: the text split into tokens
    // (words and punctuation marks), with what the model worked out about each one.
    let doc = nlp.process("Alice works at Microsoft in London.")?;

    // Print every token with its lemma and its part of speech. The lemma is the
    // dictionary form of a word ("works" becomes "work"). The part of speech is
    // the kind of word, such as VERB or PROPN (a proper noun, which is a name).
    for (index, token) in doc.tokens().iter().enumerate() {
        let text = doc.token_text(TokenIndex(index))?;
        println!("{text}\t{:?}\t{:?}", token.lemma, token.pos);
    }

    // Collect the entities. An entity is a real-world thing named in the text.
    // Each one has a label: PERSON, ORG (an organization) or GPE (a country,
    // city or state), among others.
    let mut found = Vec::new();
    for entity in doc.entities().unwrap() {
        let text = doc.span(entity.start, entity.end)?.text().to_string();
        found.push(format!("{}: {text}", entity.label));
    }

    // The model found a person, an organization and a place.
    assert_eq!(found, ["PERSON: Alice", "ORG: Microsoft", "GPE: London"]);
    println!("{found:?}");
    Ok(())
}
```

```typescript
import assert from 'node:assert/strict';
import { loadModel } from '@spars/node';

// Load the model you downloaded. Do this once and reuse it: loading is the
// slow part, and one model can read as many texts as you like.
const nlp = await loadModel('en_core_web_sm');

// Read a sentence. The result is a document: the text split into tokens
// (words and punctuation marks), with what the model worked out about each one.
const doc = await nlp.process('Alice works at Microsoft in London.');

// Print every token with its lemma and its part of speech. The lemma is the
// dictionary form of a word ("works" becomes "work"). The part of speech is
// the kind of word, such as VERB or PROPN (a proper noun, which is a name).
for (const token of doc.tokens) {
  console.log(`${token.text}\t${token.lemma}\t${token.pos}`);
}

// Collect the entities. An entity is a real-world thing named in the text.
// Each one has a label: PERSON, ORG (an organization) or GPE (a country,
// city or state), among others.
assert.ok(doc.entities);
const found = doc.entities.map(entity => {
  // An entity covers the tokens from `start` up to, but not including, `end`.
  const first = doc.tokens[entity.start];
  const last = doc.tokens[entity.end - 1];
  assert.ok(first && last);
  return `${entity.label}: ${doc.text.slice(first.utf16Start, last.utf16End)}`;
});

// The model found a person, an organization and a place.
assert.deepEqual(found, ['PERSON: Alice', 'ORG: Microsoft', 'GPE: London']);
console.log(found);
```

:::

Both programs end by printing the three entities: `PERSON: Alice`, `ORG: Microsoft` and `GPE: London`.

## What works today

SpaRs runs three English models from spaCy, version 3.8.0. All three find tokens, lemmas, parts of speech, grammar, sentences and entities.

| Model | Best for |
|---|---|
| `en_core_web_sm` | Trying SpaRs out. It is the smallest download. |
| `en_core_web_md` | Comparing the meaning of words and texts. It includes word vectors, which are lists of numbers that stand for a word's meaning. |
| `en_core_web_lg` | The same as `md` with many more distinct vectors, so fewer words share one. It is the largest download. |

[Choose a model](MODELS.md) compares them in detail.

Other languages, transformer models and training are not supported. [Compatibility](COMPATIBILITY.md) lists what is supported, and [differences from spaCy](SPACY_DIFFERENCES.md) lists where SpaRs behaves differently on purpose.

## Where to go next

- [Core concepts](CONCEPTS.md) explains tokens, lemmas, entities and the other words these guides use.
- [Choose a model](MODELS.md) compares the three English models.
- [Troubleshooting](TROUBLESHOOTING.md) explains the errors you are most likely to meet first.
- [Use SpaRs from Node.js](NODE.md) covers the whole Node.js API with examples.
- [Developer guide](DEVELOPMENT.md) has more Rust examples: sentences, offsets, vectors and running part of the pipeline.
- [Token matcher](TOKEN_MATCHER.md), [phrase matcher](PHRASE_MATCHER.md) and [dependency matcher](DEPENDENCY_MATCHER.md) find words and phrases with patterns.
- [Entity ruler](ENTITY_RULER.md) and [span ruler](SPAN_RULER.md) add your own entities and spans with rules.
- [Rust API reference](https://docs.rs/spars-nlp) lists every type and function in the Rust library.
