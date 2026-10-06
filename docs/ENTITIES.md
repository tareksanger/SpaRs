# Edit entity annotation

`Doc::set_entities` replaces a document's named-entity annotation, following spaCy 3.8.14's `Doc.set_ents`. Use it to add or correct entities after the recognizer has run, or to annotate a document that never ran it. Each token's IOB tag (`B` begins an entity, `I` continues it, `O` is outside any entity), each token's entity type and the document's entity list stay consistent after every update.

`set_entities` itself needs no model. The example below processes text first, so it needs the `en_core_web_md` export at `assets/en_core_web_md-3.8.0`, relative to the repository root, from the [development setup](DEVELOPMENT.md) or the [model installer](MODEL_INSTALLATION.md).

```rust
use spars::{EntityDefault, EntityUpdate, Model, Span, TokenIndex, TokenRange};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let nlp = Model::load("assets/en_core_web_md-3.8.0")?;
    let mut doc = nlp.process("Tim Cook visited London.")?;
    let labels = |doc: &spars::Doc| -> Vec<String> {
        doc.entities().unwrap().iter().map(|e| e.label.clone()).collect()
    };
    assert_eq!(labels(&doc), ["PERSON", "GPE"]);

    // Relabel London and keep the other predictions.
    doc.set_entities(&EntityUpdate {
        entities: vec![Span { start: TokenIndex(3), end: TokenIndex(4), label: "CITY".into() }],
        default: EntityDefault::Unmodified,
        ..EntityUpdate::default()
    })?;
    assert_eq!(labels(&doc), ["PERSON", "CITY"]);
    assert_eq!(doc.tokens()[3].entity_iob.as_deref(), Some("B"));

    // Spans that share a token are rejected and the document is unchanged.
    let overlapping = EntityUpdate {
        entities: vec![Span { start: TokenIndex(0), end: TokenIndex(2), label: "ORG".into() }],
        outside: vec![TokenRange { start: TokenIndex(1), end: TokenIndex(3) }],
        ..EntityUpdate::default()
    };
    assert!(matches!(doc.set_entities(&overlapping), Err(spars::Error::Bounds)));
    assert_eq!(labels(&doc), ["PERSON", "CITY"]);
    Ok(())
}
```

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

`Doc::entities` is `None` when a non-empty document has every token missing, meaning it has no entity annotation, which corresponds to spaCy's `doc.has_annotation("ENT_IOB")` being false. Otherwise it is the list of entities, which may be empty. An empty document always has an empty list, as spaCy counts it as annotated and `Model::process("")` returns. spaCy's `doc.ents` itself is an empty tuple in both cases.

After writing the groups and the default, an `I` tag that follows a missing or `O` token, or a token of a different type, becomes `B`. For example, marking the middle of a three-token entity as missing with `Unmodified` leaves two separate one-token entities. An `I` that follows a token of the same type is kept, so relabeling only the first token of a three-token `X` entity as `X` leaves one three-token entity. This is spaCy's repair step, applied in token order.

Tokens that the update neither writes nor repairs keep their stored values exactly. This only matters for documents restored from a native snapshot (the JSON written by `Doc::to_json` and read by `Doc::from_json`), which may spell a missing tag as `""` or store a type with it.

## Validation and errors

Every check runs before anything changes, so a rejected update leaves the document exactly as it was:

- An interval whose end is before its start or past the last token returns `Error::Bounds`. In spaCy, building such a `Span` raises `IndexError`.
- Two intervals that share a token return `Error::Bounds`. Entities with an empty label are left out of this check, and empty intervals share no tokens. spaCy raises `ValueError` (E1010).
- An `I` on the first token that the update leaves in place returns `Error::Unsupported`, because there is no entity for it to continue; any other `I` without a `B` before it is repaired as described above. Model output and earlier updates never contain this tag; only an edited native snapshot can. A current IOB tag other than `B`, `I`, `O` or missing also returns `Error::Unsupported`; `Doc::from_json` already rejects such tags, so only code that sets `Token::entity_iob` directly can produce one. For the first-token case, spaCy writes the update and raises `ValueError` (E093) when the entity list is next read; SpaRs rejects the update without changing the document.
- When deserializing an `EntityUpdate` from JSON, unknown fields, including spaCy's `kb_id`, are rejected rather than ignored.

## Ownership and views

`set_entities` takes `&mut Doc`. Rust's borrow rules prevent calling it while a `TokenView`, `SpanView` or other borrow of the document is alive, so no view can observe a half-applied update. To keep the earlier annotation, call `doc.with_entities(&update)` instead: it checks the update, then returns an updated copy and leaves the original unchanged, so an invalid update copies nothing. Text, token boundaries, offsets and all other annotations are unchanged. A native snapshot taken after an update restores the same annotation.

The [Node binding](NODE.md#edit-entity-annotation) offers only the copying form, `withEntities`, because native documents and their views are immutable and shared.

## Limits

These differ from spaCy or are not supported:

- Knowledge-base IDs (`kb_id`) and entity IDs (`ent_id`) are not stored. spaCy's `set_ents` copies them from each entity span, keeps a token's previous entity ID when the span has none, and leaves both unchanged on blocked, missing and outside tokens; SpaRs entities have only a label. Entity IDs are planned with EntityRuler support (delivery A2); knowledge-base IDs have no planned delivery.
- spaCy's `Doc.ents` setter, which also accepts `(label, start, end)` tuples and tuples carrying knowledge-base and entity IDs, is not provided; nor is the `ents` argument of spaCy's `Doc` constructor, which takes IOB strings. Use `set_entities` with token intervals.
- spaCy's `set_ents` edits the document in place; `Doc::with_entities` and the Node binding return a copy, as described above.
- Span groups (`Doc.spans`), merging or splitting tokens, and other annotation edits are not supported. The [compatibility inventory](COMPATIBILITY.md) tracks them.

## Verification

`fixtures/entity-updates-v1.expected.json` records spaCy 3.8.14's token tags, entity list and errors after 330 updates in 34 cases. One case applies four updates to `en_core_web_md` 3.8.0 predictions; the others use constructed documents. `crates/spars/tests/entity_updates.rs` replays every update through both `set_entities` and `with_entities` and checks that rejected updates change nothing; its model-dependent test checks that SpaRs predicts that case's starting entities and that updating them leaves all other annotations unchanged. `crates/spars/tests/entity_update_allocation.rs` checks that a rejected `with_entities` update does not copy the document. `bindings/node/tests/entities.test.mts` replays the same fixture through `withEntities`. `tools/test_entity_update_reference.py` regenerates the fixture with spaCy and then recomputes every recorded state from the rules above with plain Python. Run them with:

```sh
cargo test --release --offline -p spars-nlp --test entity_updates --test entity_update_allocation -- --include-ignored
npm --prefix bindings/node test
.venv/bin/python -m unittest discover -s tools -p test_entity_update_reference.py
```

The model-dependent tests need the exported models; see [validation](VALIDATION.md) for the full acceptance run.
