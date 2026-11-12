# Core concepts

This page explains the words the other guides use: document, token, lemma, part of speech, dependency, sentence, entity, span, model and pipeline. Each idea comes with a short example in Rust and in Node.js. Finish the [quick setup](index.md#quick-setup) first, so the `en_core_web_sm` model is installed.

## Text goes in, a document comes out

You give SpaRs a piece of text. It gives back a **document**: the same text, split into pieces, with notes attached to each piece. SpaRs never changes the text, so you can always get the original back.

```text
"Alice visited the old museum in Paris. She loved it."
                         │
                         ▼
                 a model reads it
                         │
                         ▼
 document: tokens, sentences, noun chunks, entities
```

The **model** is a folder of files holding the learned numbers that make this possible. SpaRs uses models trained and published by the [spaCy](https://spacy.io) team. Loading a model is the slow step, so load it once and reuse it for every text.

## Tokens

A **token** is one piece of the text: usually a word or a punctuation mark. Each token has a position, counted from 0, called its **token index**. For every token the model works out:

| Term | What it means | Example for "visited" |
|---|---|---|
| Lemma | The dictionary form of the word | `visit` |
| Part of speech | The kind of word | `VERB` |
| Tag | A more detailed kind of word | `VBD`, a verb in the past tense |
| Dependency | The job the word does in its sentence | `ROOT`, the main word of the sentence |
| Head | The token this word attaches to | itself, because it is the root |

The dependency and the head together describe the grammar of a sentence as a tree. Every token except one attaches to another token, its head. In "Alice visited the old museum", the word "museum" attaches to "visited" with the dependency `dobj` (direct object), because the museum is the thing that was visited. The one token that attaches to nothing is the main word of the sentence, its **root**. SpaRs records the root as its own head.

::: code-group

```rust
use spars::{Model, TokenIndex};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load the model once and reuse it for every text.
    let nlp = Model::load("en_core_web_sm")?;

    // Read two sentences. The tokens are:
    // Alice(0) visited(1) the(2) old(3) museum(4) in(5) Paris(6) .(7)
    // She(8) loved(9) it(10) .(11)
    let doc = nlp.process("Alice visited the old museum in Paris. She loved it.")?;

    // Look at token 1, "visited". Each note is optional (`Option`): it is missing
    // when, for example, the step that produces it did not run.
    let visited = &doc.tokens()[1];
    assert_eq!(visited.lemma.as_deref(), Some("visit"));
    assert_eq!(visited.pos.as_deref(), Some("VERB"));
    assert_eq!(visited.tag.as_deref(), Some("VBD"));

    // "visited" is the root of the first sentence, so its head is itself.
    assert_eq!(visited.dep.as_deref(), Some("ROOT"));
    assert_eq!(visited.head, Some(TokenIndex(1)));

    // Token 4, "museum", is the direct object ("dobj") of "visited".
    let museum = &doc.tokens()[4];
    assert_eq!(museum.dep.as_deref(), Some("dobj"));
    assert_eq!(museum.head, Some(TokenIndex(1)));
    Ok(())
}
```

```typescript
import assert from 'node:assert/strict';
import { loadModel } from '@spars/node';

// Load the model once and reuse it for every text.
const nlp = await loadModel('en_core_web_sm');

// Read two sentences. The tokens are:
// Alice(0) visited(1) the(2) old(3) museum(4) in(5) Paris(6) .(7)
// She(8) loved(9) it(10) .(11)
const doc = await nlp.process('Alice visited the old museum in Paris. She loved it.');

// Look at token 1, "visited". A note is `null` when, for example, the step
// that produces it did not run.
const visited = doc.tokens[1];
assert.ok(visited);
assert.equal(visited.lemma, 'visit');
assert.equal(visited.pos, 'VERB');
assert.equal(visited.tag, 'VBD');

// "visited" is the root of the first sentence, so its head is itself.
assert.equal(visited.dep, 'ROOT');
assert.equal(visited.head, 1);

// Token 4, "museum", is the direct object ("dobj") of "visited".
const museum = doc.tokens[4];
assert.ok(museum);
assert.equal(museum.dep, 'dobj');
assert.equal(museum.head, 1);
```

:::

## Spans

A **span** is a run of tokens that belong together, such as "the old museum". A span stores a start and an end token index. The end is **exclusive**: it is the index just after the last token. A span from 2 to 5 covers tokens 2, 3 and 4.

Sentences, noun chunks and entities are all spans.

## Sentences and noun chunks

The model splits a document into **sentences**. It also finds **noun chunks**: a noun or pronoun together with the words that describe it, such as "the old museum" or just "She".

::: code-group

```rust
use spars::Model;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load the model once and reuse it for every text.
    let nlp = Model::load("en_core_web_sm")?;
    let doc = nlp.process("Alice visited the old museum in Paris. She loved it.")?;

    // Each sentence is a span. `span_text` turns a span back into its text.
    let mut sentences = Vec::new();
    for sentence in doc.sentences().unwrap() {
        sentences.push(doc.span_text(sentence.start, sentence.end)?);
    }
    assert_eq!(sentences, ["Alice visited the old museum in Paris.", "She loved it."]);

    // Each noun chunk is a span too.
    let mut chunks = Vec::new();
    for chunk in doc.noun_chunks().unwrap() {
        chunks.push(doc.span_text(chunk.start, chunk.end)?);
    }
    assert_eq!(chunks, ["Alice", "the old museum", "Paris", "She", "it"]);

    // Entities are spans as well. Each one carries a label for the kind of thing
    // it names: here a person ("Alice") and a place ("Paris").
    let mut labels = Vec::new();
    for entity in doc.entities().unwrap() {
        labels.push(entity.label.as_str());
    }
    assert_eq!(labels, ["PERSON", "GPE"]);
    Ok(())
}
```

```typescript
import assert from 'node:assert/strict';
import { loadModel } from '@spars/node';

// Load the model once and reuse it for every text.
const nlp = await loadModel('en_core_web_sm');
const doc = await nlp.process('Alice visited the old museum in Paris. She loved it.');

// Turn a span back into its text: cut the document's text from where the
// first token starts to where the last token ends.
function textOf(span: { start: number; end: number }): string {
  const first = doc.tokens[span.start];
  const last = doc.tokens[span.end - 1];
  assert.ok(first && last);
  return doc.text.slice(first.utf16Start, last.utf16End);
}

// Each sentence is a span.
assert.ok(doc.sentences);
const sentences = doc.sentences.map(textOf);
assert.deepEqual(sentences, ['Alice visited the old museum in Paris.', 'She loved it.']);

// Each noun chunk is a span too.
assert.ok(doc.nounChunks);
const chunks = doc.nounChunks.map(textOf);
assert.deepEqual(chunks, ['Alice', 'the old museum', 'Paris', 'She', 'it']);

// Entities are spans as well. Each one carries a label for the kind of thing
// it names: here a person ("Alice") and a place ("Paris").
assert.ok(doc.entities);
const labels = doc.entities.map(entity => entity.label);
assert.deepEqual(labels, ['PERSON', 'GPE']);
```

:::

## Entities

An **entity** is a real-world thing that the text names: a person, a place, a company, a date. Each entity is a span with a **label** that says what kind of thing it is. In the example above the model finds `PERSON` for "Alice" and `GPE` for "Paris". The [getting started example](index.md#getting-started) shows how to read entities.

The English models use these labels. The descriptions are spaCy's own. `GPE` stands for geopolitical entity, `NORP` for nationalities or religious or political groups, `FAC` for facility and `LOC` for location.

| Label | Meaning |
|---|---|
| `PERSON` | People, including fictional |
| `NORP` | Nationalities or religious or political groups |
| `FAC` | Buildings, airports, highways, bridges, etc. |
| `ORG` | Companies, agencies, institutions, etc. |
| `GPE` | Countries, cities, states |
| `LOC` | Non-GPE locations, mountain ranges, bodies of water |
| `PRODUCT` | Objects, vehicles, foods, etc. (not services) |
| `EVENT` | Named hurricanes, battles, wars, sports events, etc. |
| `WORK_OF_ART` | Titles of books, songs, etc. |
| `LAW` | Named documents made into laws |
| `LANGUAGE` | Any named language |
| `DATE` | Absolute or relative dates or periods |
| `TIME` | Times smaller than a day |
| `PERCENT` | Percentage, including "%" |
| `MONEY` | Monetary values, including unit |
| `QUANTITY` | Measurements, as of weight or distance |
| `ORDINAL` | "first", "second", etc. |
| `CARDINAL` | Numerals that do not fall under another type |

A model's entities are its best guess. It can miss a name or pick the wrong label. To add or correct entities yourself, see [editing entities](ENTITIES.md) and the [entity ruler](ENTITY_RULER.md).

## Parts of speech

The part of speech uses the Universal POS tags, the same short codes for every language, plus spaCy's `SPACE` for whitespace tokens.

| Code | Meaning | Code | Meaning |
|---|---|---|---|
| `ADJ` | adjective | `NUM` | numeral |
| `ADP` | adposition, such as "in" or "of" | `PART` | particle |
| `ADV` | adverb | `PRON` | pronoun |
| `AUX` | auxiliary, such as "is" or "will" | `PROPN` | proper noun, a name |
| `CCONJ` | coordinating conjunction, such as "and" | `PUNCT` | punctuation |
| `DET` | determiner, such as "the" | `SCONJ` | subordinating conjunction, such as "because" |
| `INTJ` | interjection | `SYM` | symbol |
| `NOUN` | noun | `VERB` | verb |
| `SPACE` | whitespace | `X` | other |

## The pipeline

A model does its work in steps, called **stages**. The steps together are the **pipeline**. Each stage adds its own notes to the document. The name in code is shown in brackets:

| Stage | What it adds |
|---|---|
| Tokenizer (`Tokenizer`) | Splits the text into tokens |
| Tagger (`Tagger`) | The detailed tag of each token |
| Parser (`Parser`) | Dependencies and heads, and from them sentences |
| Attribute ruler (`AttributeRuler`) | The part of speech of each token. Noun chunks are available from this stage on, because they need both the grammar and the parts of speech |
| Lemmatizer (`Lemmatizer`) | The lemma of each token |
| Named entity recognizer (`Ner`) | Entities. "Ner" stands for named entity recognition |

You normally run the whole pipeline. You can also stop early, which is faster when you need only the first steps. A note from a stage that did not run is missing rather than empty: `None` in Rust and `null` in Node.js. The [developer guide](DEVELOPMENT.md#run-part-of-the-pipeline-and-save-a-document) shows how to run part of the pipeline.

## Positions in the text

Sometimes you need to know where a token sits in the original text, for example to highlight it. A position can be counted in three ways, and they differ as soon as the text contains characters such as "é" or an emoji:

- **Bytes**, the unit Rust strings use. A byte is the smallest piece of stored data.
- **Code points**, the unit Python and spaCy use. A code point is one character as Unicode numbers it.
- **UTF-16 units**, the unit JavaScript strings use.

For example, "é" is 2 bytes, 1 code point and 1 UTF-16 unit, and an emoji such as "😀" is 4 bytes, 1 code point and 2 UTF-16 units.

SpaRs gives the units separate types so they cannot be mixed up by accident: bytes and code points in Rust, and all three in Node.js. See [Unicode offsets](DEVELOPMENT.md#work-with-unicode-offsets) for Rust and the [Node.js guide](NODE.md) for JavaScript.

## Where to go next

- [Choose a model](MODELS.md) compares the three English models.
- [Troubleshooting](TROUBLESHOOTING.md) explains the errors you are most likely to meet first.
- [Token matcher](TOKEN_MATCHER.md) finds words and phrases using the notes described on this page.
