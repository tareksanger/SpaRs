# Edit entity annotation

`Doc::set_entities` replaces a document's named-entity annotation, following spaCy 3.8.14's `Doc.set_ents`. Use it to add or correct entities after the recognizer has run, or to annotate a document that never ran it. Each token's IOB tag (`B` begins an entity, `I` continues it, `O` is outside any entity), each token's entity type and the document's entity list stay consistent after every update.

`set_entities` itself needs no model. The example below processes text first, so it needs the `en_core_web_md` export at `assets/en_core_web_md-3.8.0`, relative to the repository root, from the [development setup](DEVELOPMENT.md) or the [model installer](MODEL_INSTALLATION.md).

::: code-group

```rust
use spars::{EntityDefault, EntityUpdate, Model, Span, TokenIndex, TokenRange};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load the model once. It holds everything SpaRs learned about English.
    let nlp = Model::load("assets/en_core_web_md-3.8.0")?;

    // Read the sentence. The model splits it into tokens (words and punctuation
    // marks) and finds the entities: stretches of tokens that name a real-world
    // thing. A token index is a token's position, counting from 0.
    // The tokens are: Tim(0) Cook(1) visited(2) London(3) .(4)
    let mut doc = nlp.process("Tim Cook visited London.")?;

    // A small helper that lists the label of every entity in a document, in order.
    // An entity label says what kind of thing the entity names.
    let labels = |doc: &spars::Doc| -> Vec<String> {
        doc.entities().unwrap().iter().map(|e| e.label.clone()).collect()
    };

    // The model found two entities: "Tim Cook" is a PERSON, and "London" is a GPE,
    // the label for countries, cities and states.
    assert_eq!(labels(&doc), ["PERSON", "GPE"]);

    // Relabel London and keep the other predictions.
    // An update lists the entities to write. Each one is a span: a run of tokens
    // from a start index up to an end index. The end is exclusive, so 3 to 4
    // covers only token 3, "London". `Unmodified` tells the update to leave every
    // token it does not mention as it was, which keeps "Tim Cook" a PERSON.
    doc.set_entities(&EntityUpdate {
        entities: vec![Span::new(TokenIndex(3), TokenIndex(4), "CITY")],
        default: EntityDefault::Unmodified,
        ..EntityUpdate::default()
    })?;

    // London carries the CITY label, and "Tim Cook" is still a PERSON.
    assert_eq!(labels(&doc), ["PERSON", "CITY"]);

    // Each token also has an IOB tag: B for the token that begins an entity, I for
    // a token inside one, and O for a token outside any entity. "London" is the
    // first (and only) token of its entity, so its tag is B.
    assert_eq!(doc.tokens()[3].entity_iob.as_deref(), Some("B"));

    // Spans that share a token are rejected and the document is unchanged.
    // This update is a mistake on purpose. It says tokens 0 and 1 ("Tim Cook") are
    // an ORG entity, and also that tokens 1 and 2 ("Cook visited") are outside any
    // entity. Token 1, "Cook", cannot be both.
    let overlapping = EntityUpdate {
        entities: vec![Span::new(TokenIndex(0), TokenIndex(2), "ORG")],
        outside: vec![TokenRange { start: TokenIndex(1), end: TokenIndex(3) }],
        ..EntityUpdate::default()
    };

    // Applying it returns an error instead of changing anything.
    assert!(matches!(doc.set_entities(&overlapping), Err(spars::Error::Bounds)));

    // The entities are the same as before the failed update.
    assert_eq!(labels(&doc), ["PERSON", "CITY"]);

    Ok(())
}
```

```typescript
import assert from 'node:assert/strict';
import { loadModel } from '@spars/node';
import type { NativeDocument } from '@spars/node';

// Load the model once. It holds everything SpaRs learned about English.
const nlp = await loadModel('assets/en_core_web_md-3.8.0');

// Read the sentence. The model splits it into tokens (words and punctuation
// marks) and finds the entities: stretches of tokens that name a real-world
// thing. A token index is a token's position, counting from 0.
// `processDocument` returns a native document, the kind that has `withEntities`.
// The tokens are: Tim(0) Cook(1) visited(2) London(3) .(4)
const doc = await nlp.processDocument('Tim Cook visited London.');

// A small helper that lists the label of every entity in a document, in order.
// An entity label says what kind of thing the entity names. `toObject` copies
// the document into plain JavaScript values that are easy to read.
const labels = (document: NativeDocument): string[] =>
  (document.toObject().entities ?? []).map((entity) => entity.label);

// The model found two entities: "Tim Cook" is a PERSON, and "London" is a GPE,
// the label for countries, cities and states.
assert.deepEqual(labels(doc), ['PERSON', 'GPE']);

// Relabel London and keep the other predictions.
// An update lists the entities to write. Each one is a span: a run of tokens
// from a start index up to an end index. The end is exclusive, so 3 to 4
// covers only token 3, "London". 'unmodified' tells the update to leave every
// token it does not mention as it was, which keeps "Tim Cook" a PERSON.
// A native document never changes: `withEntities` returns an edited copy.
const relabeled = doc.withEntities({
  entities: [{ start: 3, end: 4, label: 'CITY' }],
  default: 'unmodified',
});

// London carries the CITY label, and "Tim Cook" is still a PERSON.
assert.deepEqual(labels(relabeled), ['PERSON', 'CITY']);

// Each token also has an IOB tag: B for the token that begins an entity, I for
// a token inside one, and O for a token outside any entity. "London" is the
// first (and only) token of its entity, so its tag is B.
assert.equal(relabeled.token(3).annotations().entityIob, 'B');

// Spans that share a token are rejected and the document is unchanged.
// This update is a mistake on purpose. It says tokens 0 and 1 ("Tim Cook") are
// an ORG entity, and also that tokens 1 and 2 ("Cook visited") are outside any
// entity. Token 1, "Cook", cannot be both.
const overlapping = {
  entities: [{ start: 0, end: 2, label: 'ORG' }],
  outside: [{ start: 1, end: 3 }],
};

// Applying it throws an error with the code SPARS_BOUNDS and returns no copy.
assert.throws(() => relabeled.withEntities(overlapping), { code: 'SPARS_BOUNDS' });

// `relabeled` is untouched, as a native document always is.
assert.deepEqual(labels(relabeled), ['PERSON', 'CITY']);
```

:::

With `Unmodified`, "Tim Cook" keeps its predicted `PERSON` label while London becomes `CITY`. The second update would mark "Cook" both as part of an `ORG` entity and as outside any entity, so it is rejected and `CITY` stays in place.

## Token states

An `EntityUpdate` lists half-open token intervals (the end index is excluded) in four groups. Each group sets one state on its tokens:

| Group | Token IOB tag | Token entity type | Meaning |
| --- | --- | --- | --- |
| `entities` | `B` on the first token, `I` on the rest | The span label | A named entity |
| `outside` | `O` | `""` | Known not to be part of an entity |
| `blocked` | `B` | `""` | Can never be part of an entity. spaCy's recognizer respects this when it runs afterwards; SpaRs cannot yet run its recognizer on an existing document (planned as delivery A4 in the [progress plan](PROGRESS.md#2-checked-annotations-and-rule-based-annotation)), so here it only records the state |
| `missing` | `None` | `None` | Unknown, as before entity recognition |

`default` sets every token outside all four groups: `Outside` (the default), `Missing`, `Blocked`, or `Unmodified` to keep its current state. A token in the `entities` group ends up in `Doc::entities`; the other states never do. An entity with an empty label is ignored, as in spaCy, but its interval must still be inside the document. The recognizer itself writes `O` with an empty entity type, so an updated document and a predicted one represent outside tokens in the same way.

`Doc::entities` is `None` when a non-empty document has every token missing, meaning it has no entity annotation, which corresponds to spaCy's `doc.has_annotation("ENT_IOB")` being false. Otherwise it is the list of entities, which may be empty. An empty document always has an empty list, as spaCy counts it as annotated and `Model::process("")` returns. spaCy's `doc.ents` itself is an empty tuple in both cases. A document restored by `Doc::from_json` can instead have an empty list with every token missing; see [entity annotation in snapshots](#entity-annotation-in-snapshots).

After writing the groups and the default, an `I` tag that follows a missing or `O` token, or a token of a different type, becomes `B`. For example, marking the middle of a three-token entity as missing with `Unmodified` leaves two separate one-token entities. An `I` that follows a token of the same type is kept, so relabeling only the first token of a three-token `X` entity as `X` leaves one three-token entity. This is spaCy's repair step, applied in token order.

Tokens that the update neither writes nor repairs keep their stored values exactly. This only matters for documents restored from a native snapshot (the JSON written by `Doc::to_json` and read by `Doc::from_json`), which may spell a missing tag as `""` or store a type with it.

## Validation and errors

Every check runs before anything changes, so a rejected update leaves the document exactly as it was:

- An interval whose end is before its start or past the last token returns `Error::Bounds`. In spaCy, building such a `Span` raises `IndexError`.
- Two intervals that share a token return `Error::Bounds`. Entities with an empty label are left out of this check, and empty intervals share no tokens. spaCy raises `ValueError` (E1010).
- An `I` that the update leaves with no entity to continue returns `Error::Unsupported`. Snapshots cannot hold such a tag (see [entity annotation in snapshots](#entity-annotation-in-snapshots)), but an update can create one: blocking the token before an `I` that stores no entity type leaves the `I` in place, because the repair step only changes an `I` whose type differs from the token before it. Any other `I` without a `B` before it is repaired as described above. spaCy writes such an update and raises `ValueError` (E093) when the entity list is next read; SpaRs rejects it without changing the document. A current IOB tag other than `B`, `I`, `O` or missing also returns `Error::Unsupported`; model output and `Doc::from_json` never store one.
- When deserializing an `EntityUpdate` from JSON, unknown fields, including spaCy's `kb_id`, are rejected rather than ignored.

### Entity annotation in snapshots

spaCy keeps no separate entity list: `doc.ents` is always read from the token tags. A native snapshot stores both, so `Doc::from_json` checks that they agree and returns `Error::Model` otherwise. Snapshots this version writes from model output or entity updates always agree; an edited or third-party snapshot, or one saved by an earlier SpaRs version from such a snapshot, can fail these checks:

- An `I` tag with no entity to continue, on the first token or after an `O`, missing or blocked token, is rejected. spaCy's `doc.ents` raises `ValueError` (E093) for these tags.
- The stored entities must be exactly those the tags describe: each `B` tag with a non-empty entity type starts an entity, the `I` tags after it extend it, and its label is the `B` token's type. An `I` keeps extending the entity whatever type it stores, as in spaCy. An entity's ID must be its first token's. spaCy 3.8.14's `Doc.ents` getter (`spacy/tokens/doc.pyx`) is the reference for these rules.
- The entity list may be absent (`null`) only when every tag is missing, which `Doc::entities` reports as `None` ([no entity annotation](#token-states)), so an absent list cannot hide tagged entities. When every tag is missing an empty list is accepted too, and `Doc::entities` returns it as stored: spaCy's `doc.ents` is empty either way, and spaCy reference documents without entity annotation are recorded with an empty list.

Without these checks, components that read `Doc::entities`, such as an [EntityRuler](ENTITY_RULER.md) that keeps existing entities, could see entities that spaCy would not, or succeed where spaCy raises E093.

## Entity IDs

An entity can carry an ID in addition to its label. The ID is a string you choose, usually a stable identifier for the thing the entity names; for example, both "Tim Cook" and "Mr. Cook" can have the ID `"tim-cook"`. Set `Span::id` on an entity in the update; `set_entities` writes it to every token of the entity (`Token::entity_id`), and `Doc::entities` reports each entity's ID from its first token. These are spaCy's `Span.id_` and `Token.ent_id_`. The named-entity recognizer never sets IDs, so predicted entities have none until an update adds one.

This example needs no model: it builds a three-token document from a minimal native snapshot (see `Doc::from_json`). Documents from `Model::process` work the same way.

::: code-group

```rust
use spars::{Doc, EntityUpdate, Span, TokenIndex};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // A snapshot is a document saved as JSON text. This one describes the text
    // "Tim Cook spoke" and where each of its three tokens starts and ends, so no
    // model is needed to split the text.
    // The tokens are: Tim(0) Cook(1) spoke(2)
    let snapshot = r#"{"format_version":1,"document":{"text":"Tim Cook spoke","tokens":[
        {"start":0,"end":3,"idx":0,"whitespace":true,"norm":"tim"},
        {"start":4,"end":8,"idx":4,"whitespace":true,"norm":"cook"},
        {"start":9,"end":14,"idx":9,"whitespace":false,"norm":"spoke"}]}}"#;

    // Turn the snapshot into a document.
    let mut doc = Doc::from_json(snapshot)?;

    // Describe a PERSON entity over tokens 0 and 1, "Tim Cook". A span runs from
    // a start token index up to an end index that is exclusive, so 0 to 2 stops
    // before token 2.
    let mut person = Span::new(TokenIndex(0), TokenIndex(2), "PERSON");

    // Give the entity an ID: a name you choose for the thing it refers to.
    person.id = Some("tim-cook".into());

    // Write the entity to the document.
    doc.set_entities(&EntityUpdate { entities: vec![person], ..EntityUpdate::default() })?;

    // The document's first entity reports the ID.
    assert_eq!(doc.entities().unwrap()[0].id.as_deref(), Some("tim-cook"));

    // An entity without an ID keeps the ID its tokens already have, as in spaCy.
    // Here the same two tokens get the label CEO, and no ID is given.
    let relabeled = Span::new(TokenIndex(0), TokenIndex(2), "CEO");
    doc.set_entities(&EntityUpdate { entities: vec![relabeled], ..EntityUpdate::default() })?;

    // The relabeled entity still reports "tim-cook".
    assert_eq!(doc.entities().unwrap()[0].id.as_deref(), Some("tim-cook"));

    // Tokens that stop being part of an entity keep their ID too.
    // An update that lists nothing marks every token as outside any entity.
    doc.set_entities(&EntityUpdate::default())?;

    // The document has no entities left.
    assert_eq!(doc.entities().unwrap().len(), 0);

    // Token 0, "Tim", still remembers the ID it was given earlier.
    assert_eq!(doc.tokens()[0].entity_id.as_deref(), Some("tim-cook"));

    Ok(())
}
```

```typescript
import assert from 'node:assert/strict';
import { NativeDocument } from '@spars/node';

// A snapshot is a document saved as JSON text. This one describes the text
// "Tim Cook spoke" and where each of its three tokens starts and ends, so no
// model is needed to split the text.
// The tokens are: Tim(0) Cook(1) spoke(2)
const snapshot = `{"format_version":1,"document":{"text":"Tim Cook spoke","tokens":[
    {"start":0,"end":3,"idx":0,"whitespace":true,"norm":"tim"},
    {"start":4,"end":8,"idx":4,"whitespace":true,"norm":"cook"},
    {"start":9,"end":14,"idx":9,"whitespace":false,"norm":"spoke"}]}}`;

// Turn the snapshot into a document.
const doc = NativeDocument.fromSnapshot(snapshot);

// Describe a PERSON entity over tokens 0 and 1, "Tim Cook". A span runs from
// a start token index up to an end index that is exclusive, so 0 to 2 stops
// before token 2. Give the entity an ID as well: a name you choose for the
// thing it refers to.
const person = { start: 0, end: 2, label: 'PERSON', id: 'tim-cook' };

// Write the entity. A native document never changes, so `withEntities` returns
// an edited copy and leaves `doc` as it was.
const named = doc.withEntities({ entities: [person] });

// The document's first entity reports the ID. `toObject` copies the document
// into plain JavaScript values that are easy to read.
assert.equal(named.toObject().entities?.[0]?.id, 'tim-cook');

// An entity without an ID keeps the ID its tokens already have, as in spaCy.
// Here the same two tokens get the label CEO, and no ID is given.
const relabeled = named.withEntities({ entities: [{ start: 0, end: 2, label: 'CEO' }] });

// The relabeled entity still reports "tim-cook".
assert.equal(relabeled.toObject().entities?.[0]?.id, 'tim-cook');

// Tokens that stop being part of an entity keep their ID too.
// An update that lists nothing marks every token as outside any entity.
const cleared = relabeled.withEntities({});

// The document has no entities left.
assert.equal(cleared.toObject().entities?.length, 0);

// Token 0, "Tim", still remembers the ID it was given earlier.
assert.equal(cleared.token(0).annotations().entityId, 'tim-cook');
```

:::

IDs follow spaCy's rules, including two that can surprise:

- An entity without an ID, or with an empty one, does not clear IDs. Each of its tokens keeps its current ID, so a relabeled entity keeps its old ID, and a new entity without an ID reports the ID its first token already had. Give the entity an ID to replace it.
- Outside, missing and blocked tokens keep their IDs. Only `Token::entity_id` shows such an ID; `Doc::entities` never includes those tokens.

In an update, an empty ID means the same as `None`, as in spaCy. Native snapshots use format version 4 when a document has [span groups](SPAN_GROUPS.md), otherwise version 3 when any token or entity has an ID; documents without IDs or groups are still written as version 1 or 2, and older SpaRs versions reject version 3 or later instead of silently dropping the IDs. `Doc::from_json` rejects an empty ID, an ID on a sentence or noun chunk, and an entity whose ID differs from its first token's, since spaCy cannot represent those; it also checks entities against the tags, as described in [entity annotation in snapshots](#entity-annotation-in-snapshots). Knowledge-base IDs (`kb_id`) are not stored; see [limits](#limits).

## Ownership and views

`set_entities` takes `&mut Doc`. Rust's borrow rules prevent calling it while a `TokenView`, `SpanView` or other borrow of the document is alive, so no view can observe a half-applied update. To keep the earlier annotation, call `doc.with_entities(&update)` instead: it checks the update, then returns an updated copy and leaves the original unchanged, so an invalid update copies nothing. Text, token boundaries, offsets and all other annotations are unchanged. A native snapshot taken after an update restores the same annotation.

The [Node binding](NODE.md#edit-entity-annotation) offers only the copying form, `withEntities`, because native documents and their views are immutable and shared.

## Limits

These differ from spaCy or are not supported:

- Knowledge-base IDs (`kb_id`) are not stored. spaCy's `set_ents` copies each entity span's knowledge-base ID to its tokens; SpaRs has no entity linker to produce them, and they have no planned delivery. [Entity IDs](#entity-ids) are stored.
- spaCy's `Doc.ents` setter, which also accepts `(label, start, end)` tuples and tuples carrying knowledge-base and entity IDs, is not provided; nor is the `ents` argument of spaCy's `Doc` constructor, which takes IOB strings. Use `set_entities` with token intervals.
- spaCy's `set_ents` edits the document in place; `Doc::with_entities` and the Node binding return a copy, as described above.
- Merging or splitting tokens is not supported. [Span groups](SPAN_GROUPS.md) are a separate annotation and never change entities. The [compatibility inventory](COMPATIBILITY.md) tracks them.

## Verification

`fixtures/entity-updates-v1.expected.json` records spaCy 3.8.14's token tags, entity list and errors after 330 updates in 34 cases. One case applies four updates to `en_core_web_md` 3.8.0 predictions; the others use constructed documents. `crates/spars/tests/entity_updates.rs` replays every update through both `set_entities` and `with_entities` and checks that rejected updates change nothing; its model-dependent test checks that SpaRs predicts that case's starting entities and that updating them leaves all other annotations unchanged. `crates/spars/tests/entity_update_allocation.rs` checks that a rejected `with_entities` update does not copy the document. `bindings/node/tests/entities.test.mts` replays the same fixture through `withEntities`. `tools/test_entity_update_reference.py` regenerates the fixture with spaCy and then recomputes every recorded state from the rules above with plain Python. `fixtures/entity-ids-v1.expected.json` separately records spaCy's tags, types, token IDs and entity IDs after 193 updates (2 rejected) in 27 cases; its one model case starts from spaCy's recorded predictions, so replaying it needs no model. `crates/spars/tests/entity_ids.rs` and `bindings/node/tests/entity-ids.test.mts` replay it, and `tools/test_entity_id_reference.py` regenerates the fixture with spaCy, compares it with the frozen file, and recomputes every state with plain Python. The snapshot checks above are covered by `snapshots_reject_entities_spacy_would_not_read_from_the_tags` in `crates/spars/tests/robustness.rs` and the matching Node test in `bindings/node/tests/entities.test.mts`. Run them with:

```sh
cargo test --release --offline -p spars-nlp --test entity_updates --test entity_ids --test entity_update_allocation --test robustness -- --include-ignored
npm --prefix bindings/node test
.venv/bin/python -m unittest discover -s tools -p 'test_entity_*_reference.py'
```

The model-dependent tests need the exported models; see [validation](VALIDATION.md) for the full acceptance run.
