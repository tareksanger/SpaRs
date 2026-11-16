# SpaRs for Node.js

**Fast, native natural language processing for Node.js, using the official [spaCy](https://spacy.io) pretrained models.**

SpaRs reads English text and tells you what is in it: the words, their grammar, the sentences, and the names of people, places and organizations. It runs on your own machine with no Python and no network calls while it works. This package is the typed Node.js binding to the [SpaRs](https://github.com/tareksanger/SpaRs) Rust library; both use the same native code and models.

> **Built on spaCy.** SpaRs loads the pretrained models published by the [spaCy](https://spacy.io) team at [Explosion](https://explosion.ai), follows spaCy's behavior as its reference, and checks its results against spaCy's. If you work in Python, or need training, other languages or transformer models, use [spaCy](https://github.com/explosion/spaCy) itself. SpaRs is an independent project and is not made or endorsed by Explosion.

> **AI disclosure:** This project is built with AI assistance.

## Install

Use Node.js 22 or newer. Install the package and download a model with the tool that comes with it:

```sh
npm install @spars/node
npx spars download en_core_web_sm
```

The download is the only step that uses the network. It stores the model in a per-user cache folder, or in the folder named by `SPARS_MODEL_DIR` when that is set.

Release packages target Linux x64/ARM64 with glibc or musl, macOS x64/ARM64, and Windows x64/ARM64; other platforms require a source build and separate verification.

## Quick start

Save this as `quickstart.mjs` and run `node quickstart.mjs`. It prints one line per token with its text, lemma and part of speech, then checks the entity labels.

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

## Features

- **Tokens and grammar:** tokenization, lemmas, part-of-speech tags, dependency parsing, sentences and English noun chunks.
- **Named entities:** people, places, organizations and more, with [entity editing](https://tareksanger.github.io/SpaRs/ENTITIES.html).
- **Rule-based matching:** [token](https://tareksanger.github.io/SpaRs/TOKEN_MATCHER.html), [phrase](https://tareksanger.github.io/SpaRs/PHRASE_MATCHER.html) and [dependency](https://tareksanger.github.io/SpaRs/DEPENDENCY_MATCHER.html) matchers, plus an [entity ruler](https://tareksanger.github.io/SpaRs/ENTITY_RULER.html) and a [span ruler](https://tareksanger.github.io/SpaRs/SPAN_RULER.html).
- **Vectors and similarity:** word, span and document vectors, and similarity scores. Stored word vectors need `en_core_web_md` or `en_core_web_lg`; `en_core_web_sm` stores none.
- **Typed API:** the TypeScript declarations type every fixed vocabulary, such as part-of-speech tags and matcher attributes, as a set of literal values, so editors offer them as completions and the compiler rejects typos. See [types and autocomplete](https://tareksanger.github.io/SpaRs/NODE.html#types-and-autocomplete).
- **Checked against spaCy:** discrete outputs must agree exactly with the official reference on the project's [validation suites](https://tareksanger.github.io/SpaRs/VALIDATION.html).

SpaRs is experimental. It covers a bounded part of spaCy, not the whole library. It is tested for agreement with spaCy on fixed sets of text; that does not prove agreement on every input, and it makes no claim about linguistic accuracy.

## Documentation

The [documentation site](https://tareksanger.github.io/SpaRs/) holds every guide.

| Guide | What it covers |
|---|---|
| [Node.js guide](https://tareksanger.github.io/SpaRs/NODE.html) | The whole Node.js API, with examples |
| [Installing a model](https://tareksanger.github.io/SpaRs/MODEL_INSTALLATION.html) | Downloading models, where they are stored, and local archives |
| [Compatibility](https://tareksanger.github.io/SpaRs/COMPATIBILITY.html) | What is supported and what is not |
| [Differences from spaCy](https://tareksanger.github.io/SpaRs/SPACY_DIFFERENCES.html) | Where SpaRs behaves differently on purpose |
| [Validation](https://tareksanger.github.io/SpaRs/VALIDATION.html) | How results are checked against spaCy |
| [Performance](https://tareksanger.github.io/SpaRs/PERFORMANCE.html) | How to measure loading time, throughput and memory |

## Concurrency and limits

Model loading and NLP inference (`loadModel`, `process`, `processBatch`, and `pipe`) run asynchronously on background worker threads, keeping CPU-heavy inference off the event loop. Input copying and result creation still use the JavaScript thread. Each model defaults to spaCy's 1,000,000-code-point document limit (`nlp.maxLength`) and its exported batch size (`nlp.batchSize`, 256 for the supported releases). `pipe` lazily yields documents from an iterable. Inference admission (the limit on how many jobs run and wait at once) defaults to two active jobs and 32 queued jobs per package instance and JavaScript isolate (the separate JavaScript environment of the main thread or a Node worker); excess calls reject with `SPARS_BUSY`. Customize these limits with `configureExecution`; additional input caps are opt-in through `configureInputLimits`; see the [Node.js guide](https://tareksanger.github.io/SpaRs/NODE.html#server-resource-limits) for configuration, errors, and worker-pool limits. The [error-handling example](https://tareksanger.github.io/SpaRs/NODE.html#handle-processing-errors) shows a bounded response to `SPARS_BUSY`, batching independent documents within `SPARS_INPUT_LIMIT` policies, and synchronous argument errors. Admission and input policies bound queued work and input size; they do not reduce the JavaScript-thread cost of converting large accepted results.

## Upgrading from npm 0.2.0

Upgrading from npm 0.2.0 introduces bounded inference admission and a document-length limit. Applications submitting many concurrent calls or very long documents should review the [migration cases and fixes](https://tareksanger.github.io/SpaRs/NODE.html#upgrade-from-npm-0-2-0) before upgrading.

## Models and licenses

Model acquisition is explicit. Loading and inference run offline. Set `SPARS_MODEL_DIR` to share an installation directory across applications. The verified model releases are `en_core_web_sm`, `en_core_web_md`, and `en_core_web_lg` 3.8.0; this does not establish full spaCy compatibility. Model assets retain their own licenses and are acquired separately.

The package includes the project MIT license and upstream notices. Platform packages also include Cargo dependency license texts. SpaRs is independent of Explosion.
