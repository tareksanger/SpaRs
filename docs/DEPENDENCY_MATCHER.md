# Match grammatical relationships

`DependencyMatcher` finds tokens connected by dependency relationships. A pattern can identify a verb and its subject even when other words appear between them. Matches contain token indices in pattern-node order; they are not necessarily contiguous spans.

## Find verbs and subjects

Complete the model setup in the [developer guide](DEVELOPMENT.md). Register a pattern once and reuse the matcher across documents. Each node has an ID and token conditions. The first node has no link; each later node links from an earlier ID.

```rust
use spars::{DependencyMatcher, DependencyPattern, DependencyNode, DependencyLink,
    Predicate, Relation, TokenAttribute, TokenConstraint, Model, TokenIndex};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = Model::load("assets/en_core_web_md-3.8.0")?;
    let mut matcher = DependencyMatcher::new();
    matcher.add("verb_subject", vec![DependencyPattern { nodes: vec![
        DependencyNode {
            id: "verb".into(), link: None,
            constraints: vec![TokenConstraint {
                attribute: TokenAttribute::Pos,
                predicate: Predicate::Equals { value: "VERB".into() },
            }],
        },
        DependencyNode {
            id: "subject".into(),
            link: Some(DependencyLink { left: "verb".into(), relation: Relation::Child }),
            constraints: vec![TokenConstraint {
                attribute: TokenAttribute::Dep,
                predicate: Predicate::Equals { value: "nsubj".into() },
            }],
        },
    ] }])?;
    let doc = model.process("Alice works in London.")?;
    let matches = matcher.find_matches(&doc)?;
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].rule, "verb_subject");
    assert_eq!(matches[0].tokens, [TokenIndex(1), TokenIndex(0)]);
    assert_eq!(doc.token(matches[0].tokens[1])?.text(), "Alice");
    Ok(())
}
```

All constraints on a node must match. Empty constraints match any token. Node IDs must be unique, and links must refer to earlier nodes. Registering invalid patterns returns an error without changing the matcher. Adding more patterns under an existing rule name appends them; `get`, `contains`, `remove`, `len`, and `is_empty` inspect or manage registered rules.

## Supported token conditions

`TokenAttribute` supports `Text`, `Norm`, `Lemma`, `Pos`, `Tag`, `Dep`, and `Morphology`. `Predicate::Equals` compares an exact string; `In` and `NotIn` compare against a list of strings. Text comparisons are case-sensitive.

For morphology, `MorphSuperset` requires every specified feature and `MorphIntersects` requires at least one. Use strings such as `Number=Sing` or `Tense=Past`. Multi-value features are tested individually: `Case=Nom` can match `Case=Acc,Nom`. A condition containing `Case=Acc,Nom` is a complete morphology value, not two separate set conditions. Morphology `In` and `NotIn` normalize feature order; direct `Equals` retains the supplied spelling, matching the pinned reference's distinction. Empty morphology is available data, distinct from a missing annotation. Exact morphology equality uses `"_"` for that known-empty value, while membership accepts either `""` or `"_"`. Repeated morphology fields use the last value; repeated values within one field remain significant. Restored documents must use canonical morphology strings when morphology is queried; malformed or noncanonical values return errors.

The matcher validates all dependency heads and all requested annotations throughout the document before matching. Missing annotations and cycles return errors instead of an empty match list. An empty matcher returns no matches without requiring annotations. Matching uses the document alone after registration; it does not invoke model inference or download resources.

## Relationships

Relationships run from the linked earlier node to the new node. All 20 relationship operators from the pinned spaCy DependencyMatcher are supported:

| Rust relation | spaCy operator | New node |
|---|---|---|
| `Parent` / `Child` | `<` / `>` | Direct parent / child |
| `Ancestor` / `Descendant` | `<<` / `>>` | Any ancestor / descendant, excluding self |
| `ImmediatelyPrecedes` / `ImmediatelyFollows` | `.` / `;` | Next / previous token in the same dependency tree |
| `Precedes` / `Follows` | `.*` / `;*` | Any later / earlier token in the same dependency tree |
| `ImmediateRightSibling` / `ImmediateLeftSibling` | `$+` / `$-` | Sibling at the next / previous token position |
| `RightSibling` / `LeftSibling` | `$++` / `$--` | Any sibling to the right / left |
| `ImmediateRightChild` / `ImmediateLeftChild` | `>+` / `>-` | Child at the next / previous token position |
| `RightChild` / `LeftChild` | `>++` / `>--` | Any child to the right / left |
| `ImmediateRightParent` / `ImmediateLeftParent` | `<+` / `<-` | Parent at the next / previous token position |
| `RightParent` / `LeftParent` | `<++` / `<--` | Parent to the right / left |

Distinct node IDs can bind the same token when all relationships permit it. Duplicate patterns preserve duplicate matches. Results follow rule registration order, pattern order, dependency-tree groups in first-candidate order, and then token-index combinations in node order. This order is tested against official spaCy, including crossing dependencies.

## Scope and cost

The matcher compiles conditions once and uses per-call matching state. It can be shared for concurrent read-only matching. Broad patterns may have many valid combinations, so output size can still grow rapidly. Choose selective token conditions when the intended relationship allows them.

This is a typed Rust API, not an importer for arbitrary spaCy matcher JSON. Regex, fuzzy matching, custom extensions, callbacks, span inputs, integer rule IDs, additional lexical flags, and token repetition operators are not exposed yet. [Token Matcher](TOKEN_MATCHER.md) supports sequences and repetition; PhraseMatcher remains planned. Pattern JSON uses the Rust types' fields and rejects unknown fields or enum values. Full spaCy matcher compatibility is not claimed.
