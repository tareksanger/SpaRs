# Span groups

A span group is a named list of spans stored in a document, following spaCy 3.8.14's `Doc.spans` and `SpanGroup`. Unlike entities, the spans in a group may overlap, repeat, or be empty, so groups can hold candidate mentions, alternative analyses, or the output of rules that should not overwrite entities. The Rust and Node [SpanRuler](SPAN_RULER.md) writes its matches to a span group. Span groups are independent of entities, sentences and noun chunks: setting a group does not change any other annotation. This guide shows its example in Rust and Node.js and describes the Rust API of the `spars-nlp` package, imported as `spars` (see the [README](../README.md)); Node exposes span groups through `NativeDocument.withSpanGroup`, `withoutSpanGroup` and `toObject().spanGroups`, as described in [the Node guide](NODE.md#store-span-groups).

## Store and read groups

`Doc::span_groups` returns the groups by key. `Doc::set_span_group` stores a group under a key, and `Doc::remove_span_group` removes one. A [`SpanGroup`](../crates/spars/src/document/span_groups.rs) has a `name`, JSON `attrs` and its `spans`. The example builds a three-token document from a native snapshot, so it needs no model: each token gives its byte offsets `start` and `end`, its code-point offset `idx`, whether a space follows it, and its norm, and `null` marks annotations that were not produced. [Entity editing](ENTITIES.md) describes snapshot validation.

::: code-group

```rust
use spars::{Doc, Error, Span, SpanGroup, TokenIndex};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // A small helper that writes the JSON for one token: where it starts and ends
    // in the text, and whether a space follows it. A token is one piece of the
    // text, usually a word or a punctuation mark.
    let token = |start: usize, end: usize, space: bool| format!(
        r#"{{"start":{start},"end":{end},"idx":{start},"whitespace":{space},"norm":"x"}}"#);

    // Build a document from a snapshot, which is a document saved as JSON text.
    // No model is needed because the snapshot already says where each token is.
    // A token index is a token's position in the document, counting from 0.
    // The tokens are: New(0) York(1) City(2)
    let mut doc = Doc::from_json(&format!(
        r#"{{"format_version":1,"document":{{"text":"New York City","tokens":[{},{},{}],
        "entities":null,"sentences":null,"noun_chunks":null}}}}"#,
        token(0, 3, true), token(4, 8, true), token(9, 13, false)))?;

    // A small helper that makes a span: a run of tokens with a label. It runs from
    // a start token index up to an end index. The end is exclusive, so 0 to 2
    // covers tokens 0 and 1.
    let span = |start, end, label: &str| Span::new(TokenIndex(start), TokenIndex(end), label);

    // Overlapping spans are allowed, and spans keep the order they were added in.
    // A span group is a named list of spans. This one holds "New York City" and
    // "New York", both labeled GPE, the label for countries, cities and states.
    let mut places = SpanGroup::new("places", vec![span(0, 3, "GPE"), span(0, 2, "GPE")]);

    // Attach a note to the group saying where the spans came from. The value is
    // written as JSON text, so a string keeps its double quotes.
    places.attrs.insert("source", r#""gazetteer""#)?;

    // Store the group in the document under the key "places".
    doc.set_span_group("places", places)?;

    // Read the group back from the document by its key.
    let group = doc.span_groups().get("places").unwrap();

    // The two spans share tokens 0 and 1, so the group reports an overlap.
    assert!(group.has_overlap());

    // The note is stored exactly as it was written.
    assert_eq!(group.attrs.get("source"), Some(r#""gazetteer""#));

    // To edit a group, copy it, change it and store it again. Replacing a key keeps its position.
    // First store a second, empty group so the document has two keys.
    doc.set_span_group("other", SpanGroup::new("other", vec![]))?;

    // Copy the "places" group, add a span over token 2, "City", and store the
    // copy under the same key. This replaces the group stored there before.
    let mut edited = doc.span_groups().get("places").unwrap().clone();
    edited.spans.push(span(2, 3, "CITY"));
    doc.set_span_group("places", edited)?;

    // "places" is still first, even though it was stored again after "other".
    assert_eq!(doc.span_groups().keys().collect::<Vec<_>>(), ["places", "other"]);

    // Spans must lie within the document; a rejected group leaves the document unchanged.
    // This span would end at index 4, but the document has only three tokens.
    let outside = SpanGroup::new("bad", vec![span(2, 4, "X")]);

    // Storing the group returns an error.
    assert!(matches!(doc.set_span_group("bad", outside), Err(Error::Bounds)));

    // The document has no group under the key "bad".
    assert!(doc.span_groups().get("bad").is_none());

    // Snapshots keep every group, its attrs and the key order.
    // Save the document as JSON text, then read that text back into a document.
    let restored = Doc::from_json(&doc.to_json()?)?;

    // The document read back is equal to the one that was saved.
    assert_eq!(restored, doc);

    // Removing a group hands back the group that was removed.
    assert_eq!(doc.remove_span_group("other").unwrap().name, "other");

    Ok(())
}
```

```typescript
import assert from 'node:assert/strict';
import { NativeDocument } from '@spars/node';
import type { SpanInput } from '@spars/node';

// A small helper that writes the JSON for one token: where it starts and ends
// in the text, and whether a space follows it. A token is one piece of the
// text, usually a word or a punctuation mark.
const token = (start: number, end: number, space: boolean): string =>
  `{"start":${start},"end":${end},"idx":${start},"whitespace":${space},"norm":"x"}`;

// Build a document from a snapshot, which is a document saved as JSON text.
// No model is needed because the snapshot already says where each token is.
// A token index is a token's position in the document, counting from 0.
// The tokens are: New(0) York(1) City(2)
const doc = NativeDocument.fromSnapshot(
  `{"format_version":1,"document":{"text":"New York City","tokens":[
  ${token(0, 3, true)},${token(4, 8, true)},${token(9, 13, false)}],
  "entities":null,"sentences":null,"noun_chunks":null}}`);

// A small helper that makes a span: a run of tokens with a label. It runs from
// a start token index up to an end index. The end is exclusive, so 0 to 2
// covers tokens 0 and 1.
const span = (start: number, end: number, label: string): SpanInput => ({ start, end, label });

// Overlapping spans are allowed, and spans keep the order they were added in.
// A span group is a named list of spans. This one holds "New York City" and
// "New York", both labeled GPE, the label for countries, cities and states.
// `attrs` attaches a note to the group saying where the spans came from.
// Node takes ordinary JavaScript values here, not JSON text.
// A native document never changes, so `withSpanGroup` returns a copy that
// stores the group under the key "places".
const stored = doc.withSpanGroup('places', {
  name: 'places',
  attrs: { source: 'gazetteer' },
  spans: [span(0, 3, 'GPE'), span(0, 2, 'GPE')],
});

// Read the group back from the document by its key. `toObject` copies the
// document into plain JavaScript values, with the groups as a list.
const group = stored.toObject().spanGroups.find((entry) => entry.key === 'places');
assert.ok(group);

// The two spans share tokens 0 and 1, so they overlap. Node has no overlap
// check on a group, so compare the two spans directly: each one starts
// before the other ends.
const [whole, part] = group.spans;
assert.ok(whole);
assert.ok(part);
assert.ok(whole.start < part.end && part.start < whole.end);

// The note comes back as the value that was stored.
assert.equal(group.attrs['source'], 'gazetteer');

// To edit a group, copy it, change it and store it again. Replacing a key keeps
// its position. First store a second, empty group so the document has two keys.
const withOther = stored.withSpanGroup('other', { name: 'other', spans: [] });

// The `group` read above is already a plain copy. Add a span over token 2,
// "City", and store it under the same key. This replaces the group stored
// there before.
const edited = withOther.withSpanGroup('places', {
  name: group.name,
  attrs: group.attrs,
  spans: [...group.spans, span(2, 3, 'CITY')],
});

// "places" is still first, even though it was stored again after "other".
assert.deepEqual(edited.toObject().spanGroups.map((entry) => entry.key), ['places', 'other']);

// Spans must lie within the document; a rejected group leaves the document
// unchanged. This span would end at index 4, but the document has only three
// tokens, so storing the group throws an error with the code SPARS_BOUNDS.
assert.throws(
  () => edited.withSpanGroup('bad', { name: 'bad', spans: [span(2, 4, 'X')] }),
  { code: 'SPARS_BOUNDS' });

// `edited` is untouched, as a native document always is: it has no "bad" group.
assert.ok(!edited.toObject().spanGroups.some((entry) => entry.key === 'bad'));

// Snapshots keep every group, its attrs and the key order.
// Save the document as JSON text, then read that text back into a document.
const restored = NativeDocument.fromSnapshot(edited.toSnapshot());

// A native document has no equality check, so compare the plain copies that
// `toObject` returns.
assert.deepEqual(restored.toObject(), edited.toObject());

// Removing a group returns a copy without it; the "places" group stays.
const remaining = edited.withoutSpanGroup('other');
assert.deepEqual(remaining.toObject().spanGroups.map((entry) => entry.key), ['places']);
```

:::

## Behavior

These follow spaCy unless noted here or listed under [differences from spaCy](#differences-from-spacy).

- **Keys and names.** A group's `name` need not equal its key. spaCy names a group after its key when a plain list of spans is assigned (`doc.spans["k"] = [...]`); the equivalent here is `SpanGroup::new(key, spans)`. A new key goes after the existing keys, and replacing a key keeps its position, as in a Python dict.
- **Spans.** Each span covers tokens `start..end`, where `start <= end` and `end` is at most the number of tokens; `start == end` is an empty span. Labels may be empty. A span's `id` is spaCy's `Span.id_`; `Doc::set_span_group` stores an empty ID as `None`, since spaCy uses `""` for no ID. Bounds are checked when the group is stored: `Doc::set_span_group` returns `Error::Bounds` for any span with `start > end` or outside the document, where spaCy raises `IndexError` (E035) when the `Span` is constructed (slicing with `doc[a:b]` clamps instead).
- **Attributes.** `attrs` holds JSON values under string keys in insertion order, like spaCy's `SpanGroup.attrs` dict restricted to JSON values; spaCy also accepts other values its msgpack serializer can save, which SpaRs cannot store. `SpanGroupAttrs::insert` takes a value as JSON text, such as `"1"` or `r#""text""#`, and returns `Error::Json` for text that is not one JSON value. Values are stored as the text given, so the key order inside nested objects is kept, and two values are equal only when their texts are; `{"a": 1}` and `{"a":1}` differ.
- **Overlap.** `SpanGroup::has_overlap` reports whether two spans share a token, like `SpanGroup.has_overlap`. An empty span overlaps a span that starts before it and ends after it, but not one that only touches it.
- **Combining groups.** `SpanGroup::extend_group` appends another group's spans and adds the attrs whose keys the group lacks, like `SpanGroup.extend` with a group. `SpanGroup::concat` returns a new group with the first group's name, like `+`. Appending or extending with spans is ordinary `Vec` editing of `spans`.
- **Snapshots.** A document with span groups is saved as snapshot format version 4, which older SpaRs versions reject instead of dropping the groups. Documents without groups are saved in the oldest version that holds their data. `Doc::from_json` returns `Error::Bounds` for spans with `start > end` or outside the document, `Error::Model` for repeated group keys and empty span IDs, `Error::Json` for repeated attribute keys, attrs that are not a JSON object and unknown fields in a group entry, and `Error::Unsupported` for groups in a snapshot older than version 4.

## Differences from spaCy

These differences are [accepted](SPACY_DIFFERENCES.md#span-groups) as SpaRs's intended behavior; that page gives the reason and effect of each.

- **Removing a span by index.** spaCy 3.8.14's `del group[i]` removes the span before index `i` (an upstream bug). Here `group.spans.remove(i)` removes the span at index `i`.
- **Groups are values.** spaCy returns the stored group object, so editing it changes the document. Here callers edit a copy and store it with `Doc::set_span_group`, and each key holds its own group.
- **Copies keep empty spans.** spaCy's `Doc.copy()` raises E1052 for an empty span at the start or end of the document or after a token with trailing whitespace. `Doc::clone` copies them.
- **Snapshots keep key order.** spaCy's `Doc.to_bytes` can reorder keys whose groups have identical content. SpaRs snapshots keep the key order.
- **Attributes are JSON values compared as text.** spaCy accepts any value msgpack can save and compares attrs as dicts. SpaRs stores JSON values and compares their keys in order and their text.
- **Spans and groups from another document.** spaCy rejects them. SpaRs accepts a group whose spans fit the document.
- **No knowledge-base IDs.** spaCy group members carry a `kb_id`; SpaRs spans do not.
- **Extending a group with itself.** spaCy's `group.extend(group)` is undefined behavior in C++. `extend_group` with a copy appends its spans once.

## Verification

`fixtures/span-groups-v1.expected.json` freezes spaCy 3.8.14 span group states after assigning lists and groups, deleting keys, appending, extending with spans and groups, concatenating, replacing spans by index, and copying documents, including rejected edits. `crates/spars/tests/span_groups.rs` replays every step, compares the keys, names, attrs, spans and `has_overlap` of each group and the rejected edits, checks that a rejected edit changes nothing, and checks a snapshot round trip after every step. `bindings/node/tests/span-groups.test.mts` replays every step through the Node binding. `tools/test_span_group_reference.py` regenerates the fixture with spaCy, compares it with the frozen file, and replays every step with plain Python. Run them with:

```bash
cargo test --release --offline -p spars-nlp --test span_groups
cargo test --release --offline -p spars-nlp --lib span_groups
.venv/bin/python -m unittest discover -s tools -p test_span_group_reference.py
npm --prefix bindings/node test
```

The Python check needs the [reference environment](DEVELOPMENT.md) with `en_core_web_md` 3.8.0 installed. The Node suite needs the built binding, as described in [the Node guide](NODE.md); its span group tests need no model.
