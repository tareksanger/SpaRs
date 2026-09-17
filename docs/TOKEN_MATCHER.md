# Match sequences of tokens

`TokenMatcher` finds contiguous token sequences using text or linguistic annotations. Register patterns once, then reuse the matcher across documents. Each result names the rule and gives a start token index and an exclusive end token index.

## Find a verb followed by a place

Complete the model setup in the [developer guide](DEVELOPMENT.md). This example matches the lemma “visit” followed by one or more proper nouns. It finds “visited New York” even though the text uses a different verb form.

```rust
use spars::{Model, Predicate, Repetition, TokenAttribute, TokenConstraint,
    TokenIndex, TokenMatcher, TokenPattern, TokenPatternItem};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = Model::load("assets/en_core_web_md-3.8.0")?;
    let mut matcher = TokenMatcher::new();
    matcher.add("visit_place", vec![TokenPattern { tokens: vec![
        TokenPatternItem {
            constraints: vec![TokenConstraint {
                attribute: TokenAttribute::Lemma,
                predicate: Predicate::Equals { value: "visit".into() },
            }],
            repetition: Repetition::Once,
        },
        TokenPatternItem {
            constraints: vec![TokenConstraint {
                attribute: TokenAttribute::Pos,
                predicate: Predicate::Equals { value: "PROPN".into() },
            }],
            repetition: Repetition::OneOrMore,
        },
    ] }])?;
    let doc = model.process("Alice visited New York.")?;
    let matches = matcher.find_matches(&doc)?;
    assert!(matches.iter().any(|m| m.rule == "visit_place"
        && m.start == TokenIndex(1) && m.end == TokenIndex(4)));
    let complete = matches.iter().find(|m| m.end == TokenIndex(4)).unwrap();
    assert_eq!(doc.span_text(complete.start, complete.end)?, "visited New York");
    Ok(())
}
```

The default returns overlapping matches, so “visited New” also matches this pattern. A pattern describes token conditions; it does not verify that a name is a place. Use entity annotations separately when that distinction matters.

## Conditions and repetition

All conditions on an item must match the same token. Empty conditions match any token. Token Matcher shares `TokenConstraint`, `TokenAttribute`, and `Predicate` with [DependencyMatcher](DEPENDENCY_MATCHER.md), including its morphology rules. Available attributes are text, normalization, lemma, POS, tag, dependency label, and morphology. Comparisons support equality, membership, exclusion, and morphology superset or intersection. Text equality is case-sensitive.

| Repetition | spaCy operator | Meaning |
|---|---|---|
| `Once` | omitted | Exactly one matching token |
| `Optional` | `?` | Zero or one matching token |
| `ZeroOrMore` | `*` | Any number of matching tokens |
| `OneOrMore` | `+` | At least one matching token |
| `Negated` | `!` | One token that fails the combined conditions |
| `Range { min, max: Some(max) }` | `{min,max}` | Between the two counts, inclusive |
| `Range { min, max: None }` | `{min,}` | At least `min` matching tokens |

Negation consumes a token; it is not a check between tokens. Zero-length results are omitted. Matching may cross sentence boundaries. Start and end values are token indices, not byte or character offsets; use `doc.span` or `doc.span_text` to access the original text safely.

## Reuse, ordering, and errors

`add` validates all supplied patterns before changing the matcher. Adding an existing rule appends its patterns. `get`, `contains`, `remove`, `len`, and `is_empty` manage rules. Calls use separate matching state, so registered matchers can be shared for concurrent read-only use. Matching does not invoke inference or access the network.

Results preserve the pinned spaCy matcher's discovery order, including optional suffixes at the end of the document. They are not sorted by start position or grouped by rule. Identical `(rule, start, end)` results appear once, even when several patterns or repetition paths produce them. Different rule names can match the same span.

Requested annotations must be available throughout the document. Missing annotations return errors, including when another condition would have ruled out the token. Items with an exact zero count are discarded and do not require annotations. Text-only matching does not require dependency heads or linguistic annotations. Invalid patterns and unsupported serialized enum values return errors.

## Scope and cost

This API implements the default overlapping-match behavior. Greedy `FIRST` and `LONGEST` selection, alignments, callbacks, regex and fuzzy predicates, custom extensions, additional lexical attributes, and span input are not exposed. It is a typed Rust API, not an importer for arbitrary spaCy pattern JSON. PhraseMatcher is a separate planned feature.

Patterns are limited to 4,096 expanded nodes to keep registration bounded. `OneOrMore` uses two nodes; a bounded range uses its maximum count, and an unbounded range uses its minimum plus one. Larger patterns return an error.

Broad repetition can produce a quadratic number of spans. Several optional items can also reach the same state by different paths. Prefer specific conditions and bounded repetition when they express the intended pattern. Performance measurements must include both the input length and the number of returned matches.
