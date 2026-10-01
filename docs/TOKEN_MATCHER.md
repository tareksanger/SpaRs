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

## Match without regard to case

`TokenAttribute::Lower` compares the lowercase form of each token's text, like spaCy's `LOWER`. This example finds “the Ritz” however it is capitalized.

```rust
use spars::{Model, Predicate, TokenAttribute, TokenConstraint, TokenIndex,
    TokenMatcher, TokenPattern, TokenPatternItem};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = Model::load("assets/en_core_web_md-3.8.0")?;
    let lower = |value: &str| TokenPatternItem {
        constraints: vec![TokenConstraint {
            attribute: TokenAttribute::Lower,
            predicate: Predicate::Equals { value: value.into() },
        }],
        repetition: Default::default(),
    };
    let mut matcher = TokenMatcher::new();
    matcher.add("ritz", vec![TokenPattern { tokens: vec![lower("the"), lower("ritz")] }])?;
    let doc = model.process("THE RITZ is near the Ritz.")?;
    let found: Vec<_> = matcher.find_matches(&doc)?.iter()
        .map(|m| (m.start, m.end)).collect();
    assert_eq!(found, [(TokenIndex(0), TokenIndex(2)), (TokenIndex(4), TokenIndex(6))]);
    Ok(())
}
```

Pattern values are compared exactly as written; they are not lowercased. A value such as `"Ritz"` therefore never matches, because lowercasing always turns `R` into `r`. Write pattern values in lowercase. Lowercase mappings follow Python's Unicode 15.0.0 `str.lower()`. This includes the Greek final sigma rule: a capital `Σ` becomes `ς` only when a cased letter comes before it and no cased letter follows it, ignoring combining marks between them; otherwise it becomes `σ`. So `ΟΣ` becomes `ος`, while a lone `Σ` becomes `σ`. This is lowercasing, not case folding or Unicode normalization: `ß` does not become `ss`, and a single-character `é` stays distinct from an `e` followed by a separate combining accent. `Lower` needs only token text, not model annotations, and it differs from `Norm`, which can replace a word such as `Gon` with `going`. It uses the same pinned lowercase data as PhraseMatcher `LOWER`. Only `Equals`, `In` and `NotIn` work with `Lower`. spaCy also accepts the set comparisons `IS_SUBSET`, `IS_SUPERSET` and `INTERSECTS` on `LOWER`; here set comparisons exist only as `MorphSuperset` and `MorphIntersects` on `Morphology`, so using them with `Lower` returns a pattern error.

## Match lexical flags

Lexical flags describe the form of a token's text, like spaCy's `IS_ALPHA`, `IS_DIGIT`, `IS_SPACE`, `IS_PUNCT` and `LIKE_NUM`. Some depend on the language: English `LIKE_NUM` accepts number words such as “ten” and ordinals such as “3rd”. A matcher therefore needs the model's lexicon, its language data such as Unicode character tables and number words, just as spaCy creates a matcher from the model's vocabulary. The supported models are English, and `LikeNum` implements spaCy's English rule. This example finds a number followed by a word.

```rust
use spars::{Model, Predicate, TokenAttribute, TokenConstraint, TokenIndex,
    TokenMatcher, TokenPattern, TokenPatternItem};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = Model::load("assets/en_core_web_md-3.8.0")?;
    let flag = |attribute| TokenPatternItem {
        constraints: vec![TokenConstraint { attribute, predicate: Predicate::Flag { value: true } }],
        repetition: Default::default(),
    };
    let mut matcher = TokenMatcher::with_lexicon(model.lexicon());
    matcher.add("quantity", vec![TokenPattern {
        tokens: vec![flag(TokenAttribute::LikeNum), flag(TokenAttribute::IsAlpha)],
    }])?;
    let doc = model.process("Ten people paid 1,000 dollars.")?;
    let found: Vec<_> = matcher.find_matches(&doc)?.iter()
        .map(|m| (m.start, m.end)).collect();
    assert_eq!(found, [(TokenIndex(0), TokenIndex(2)), (TokenIndex(3), TokenIndex(5))]);
    Ok(())
}
```

It finds “Ten people” (tokens 0–2) and “1,000 dollars” (tokens 3–5). “Ten” is an English number word, and the comma in “1,000” is ignored. `IsAlpha` requires letters only, so “1,000” itself is not a word.

`IsAlpha`, `IsDigit` and `IsSpace` match Python's `str.isalpha()`, `str.isdigit()` and `str.isspace()`. So the superscript `²` counts as a digit but the fraction `½` does not, and the Roman numeral `Ⅻ` and a decomposed `é` (an `e` followed by a combining accent) are not alphabetic. `IsPunct` follows spaCy: every character must be in a Unicode punctuation category, so `@` and `_` are punctuation but `$` and `+` are not. English `LikeNum` removes one leading `+`, `-`, `±` or `~` and all `,` and `.` characters, then accepts digits, two digit strings joined by one `/` such as `1/2`, number and ordinal words in any capitalization such as `ten` and `Tenth`, and digits followed by `st`, `nd`, `rd` or `th`, even `3th`. It rejects `1.5e3`, `1/2/3` and hyphenated words such as `twenty-one`.

Flag attributes accept only `Predicate::Flag { value }` with `true` or `false`. In JSON the predicate is `{"kind": "flag", "value": true}`; strings, numbers and `null` are rejected. spaCy's pattern validation (`validate=True`) rejects strings and numbers for these attributes, and spaCy raises an error for `null` when the pattern is added. Flags are computed from token text, so they need no model annotations. `TokenMatcher::new()` still works for all other conditions; registering a flag condition on it returns a pattern error. The lexicon handle is cheap to clone and stays valid after the model is dropped. Other lexical attributes, such as `IS_UPPER`, `IS_STOP`, `LIKE_URL` and `LENGTH`, are not supported yet.

## Conditions and repetition

All conditions on an item must match the same token. Empty conditions match any token. Token Matcher shares `TokenConstraint`, `TokenAttribute`, and `Predicate` with [DependencyMatcher](DEPENDENCY_MATCHER.md), including its morphology rules. Available attributes are `Text`, `Lower` (lowercase text), `Norm`, `Lemma`, `Pos`, `Tag`, `Dep`, `Morphology`, and the lexical flags `IsAlpha`, `IsDigit`, `IsSpace`, `IsPunct` and `LikeNum`. Comparisons support equality, membership, exclusion, morphology superset or intersection, and true/false flags. Text equality is case-sensitive; to ignore capitalization in the document, use `Lower` with lowercase pattern values.

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

Requested annotations must be available throughout the document. Missing annotations return errors, including when another condition would have ruled out the token. Items with an exact zero count are discarded and do not require annotations. Text-only, lowercase and lexical flag matching do not require dependency heads or linguistic annotations. Invalid patterns and unsupported serialized enum values return errors.

## Scope and cost

This API implements the default overlapping-match behavior. Greedy `FIRST` and `LONGEST` selection, alignments, callbacks, regex and fuzzy predicates, custom extensions, lexical attributes other than the five flags, and span input are not exposed. It is a typed Rust API, not an importer for arbitrary spaCy pattern JSON. [PhraseMatcher](PHRASE_MATCHER.md) supports exact or Unicode-lowercase token phrase patterns.

Patterns are limited to 4,096 expanded nodes to keep registration bounded. `OneOrMore` uses two nodes; a bounded range uses its maximum count, and an unbounded range uses its minimum plus one. Larger patterns return an error.

Broad repetition can produce a quadratic number of spans. Several optional items can also reach the same state by different paths. Prefer specific conditions and bounded repetition when they express the intended pattern. Performance measurements must include both the input length and the number of returned matches.
