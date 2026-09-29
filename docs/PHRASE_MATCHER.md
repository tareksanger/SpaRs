# Match token phrases

`PhraseMatcher` compiles tokenized document patterns into a reusable prefix tree. It compares exact token text with `ORTH`/`TEXT`, or pinned Unicode lowercase with `LOWER`. Matching itself requires neither a model nor linguistic annotations, and never downloads assets or runs inference. Patterns and input may come from different models or validated native snapshots; text identity does not depend on a shared vocabulary.

## Register and match

Complete the model setup in the [developer guide](DEVELOPMENT.md) to run this example. Tokenization is explicit here; applications with existing documents can register and search those directly.

```rust
use spars::{Model, PhraseAttribute, PhraseMatcher, PhraseRuleId, Stage, TokenIndex};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = Model::load("assets/en_core_web_md-3.8.0")?;
    let short = model.process_until("New", Stage::Tokenizer)?;
    let long = model.process_until("New York", Stage::Tokenizer)?;
    let input = model.process_until("New York and New York", Stage::Tokenizer)?;
    let mut matcher = PhraseMatcher::with_attribute(PhraseAttribute::Orth);
    matcher.add("place", &[&short, &long, &long])?;
    assert_eq!(matcher.len(), 1);
    assert_eq!(matcher.get("place").unwrap().len(), 2);
    let matches = matcher.find_matches(&input)?;
    assert_eq!(matches.len(), 4);
    assert_eq!(matches[0].rule, PhraseRuleId::from("place"));
    assert_eq!((matches[0].start, matches[0].end), (TokenIndex(0), TokenIndex(1)));
    assert_eq!(input.span_text(matches[1].start, matches[1].end)?, "New York");
    matcher.remove("place")?;
    assert!(matcher.find_matches(&input)?.is_empty());
    Ok(())
}
```

The result includes both “New” and “New York” at each occurrence. End indices are exclusive token indices, not byte or character offsets. Trailing spaces between tokens do not participate in comparisons; a whitespace token does. Sentence boundaries do not prevent a match.

## Match lowercase token text

`PhraseAttribute::Lower` applies the pinned Python Unicode 15.0.0 lowercase mapping to each complete token in both patterns and input. This includes context-sensitive Greek final sigma and multi-character mappings such as `İ` to `i` plus a combining dot. It does not perform case folding or Unicode normalization: `ß` and `ss`, and composed `é` and decomposed `e` plus an accent, remain different. Original document text and offsets remain unchanged.

```rust
use spars::{Model, PhraseAttribute, PhraseMatcher, Stage, TokenIndex};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = Model::load("assets/en_core_web_md-3.8.0")?;
    let pattern = model.process_until("the ritz", Stage::Tokenizer)?;
    let input = model.process_until("The Ritz", Stage::Tokenizer)?;
    let mut matcher = PhraseMatcher::with_attribute(PhraseAttribute::Lower);
    matcher.add("hotel", &[&pattern])?;
    let matches = matcher.find_matches(&input)?;
    assert_eq!(matches.len(), 1);
    assert_eq!((matches[0].start, matches[0].end), (TokenIndex(0), TokenIndex(2)));
    assert_eq!(input.span_text(matches[0].start, matches[0].end)?, "The Ritz");
    assert_eq!(matcher.get("hotel").unwrap()[0].tokens(), &["the", "ritz"]);
    Ok(())
}
```

`get` returns the stored lowercase token strings for a LOWER matcher; patterns with the same lowercase token sequence are deduplicated. The standalone resource is bundled with the Rust crate, so matching restored documents does not require loading a model. `crates/spars/tests/phrase_matcher_lower.rs` and the Node phrase suite compare 4 official cases, 24 lifecycle states and 351 ordered matches. Regenerate and compare both the resource and fixture with `.venv/bin/python tools/phrase_lower_reference.py --check`.

## Ordering and rule lifecycle

`new()` selects `PhraseAttribute::Orth`; `Text` is the synonymous exact-text choice. Adding an existing rule accumulates patterns. Identical pattern/rule pairs emit only one occurrence; different rules may match the same span. Empty documents used as patterns are ignored, but an empty pattern collection still registers the rule. `contains`, `len`, `is_empty` and `remove` manage rules. Removing an unknown name returns an error. `get` is a Rust inspection API returning unique nonempty patterns in first-registration order; spaCy PhraseMatcher has no corresponding public `get` method.

Results follow spaCy 3.8.14: increasing start index, then increasing end index, then preshed 3.0.13 terminal-table order for rules on the same span. The final tie order depends on label hashes, table growth, deletions and re-registration. It is neither alphabetical nor insertion order. Empty and reserved-symbol rule names are supported. Matchers can be shared across concurrent read-only calls; every call owns its result vector.

The [frozen reference corpus](../fixtures/README.md#exact-text-phrase-matching) covers 43 cases, 1,249 lifecycle states and 31,919 ordered outputs. Run `cargo test --release --offline --test phrase_matcher -- --nocapture` for explicit denominators. The acceptance script regenerates official expectations separately and runs this guide from its Markdown source.

## Rust boundaries and remaining scope

Patterns are borrowed, validated `Doc` objects; the matcher copies their selected token text, applying lowercase for LOWER. Python's raw integer arrays, invalid dynamic objects, callbacks and integer rule IDs are not accepted. Rust validates fallible inputs before changing registration state; spaCy may register a rule and earlier patterns before an invalid later item raises an error. Invalid native snapshots fail at `Doc::from_json`, before registration. Distinct rule names that collide in their resolved 64-bit identity return an error instead of sharing an upstream identity. Token text uses full string equality rather than accepting hash collisions or truncating a pattern when a token hash equals spaCy's reserved end-of-phrase marker. These are explicit safer Rust differences, not parity claims for those inputs.

Annotation attributes, validation warnings, span input, labeled-span output, matcher serialization and callbacks are not exposed. Unsupported serialized attribute names are rejected. The [Node binding](NODE.md) exposes the same ORTH/TEXT/LOWER matching and rule lifecycle through immutable native documents. Shared attributes and options, and EntityRuler/SpanRuler, remain in the [implementation sequence](PROGRESS.md#implementation-sequence).

Compilation and retained storage grow with the total pattern text and shared-prefix tree. Search costs up to document length times the longest matching prefix, plus terminal-table scans and output size; dense overlaps may return many results. A terminal table retains its capacity while any label remains, so matching a heavily pruned dictionary can cost more than matching a freshly built equivalent. Removal prunes unused nodes and releases their contents, while the node arena retains reusable slots. See [performance measurements](PERFORMANCE.md) for reproducible measurement guidance.
