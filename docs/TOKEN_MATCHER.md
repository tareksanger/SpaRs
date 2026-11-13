# Match sequences of tokens

`TokenMatcher` finds contiguous token sequences using text or linguistic annotations. Register patterns once, then reuse the matcher across documents. Each result names the rule and gives a start token index and an exclusive end token index.

## Find a verb followed by a place

Complete the model setup in the [developer guide](DEVELOPMENT.md). This example matches the lemma “visit” followed by one or more proper nouns. It finds “visited New York” even though the text uses a different verb form.

::: code-group

```rust
use spars::{Model, Predicate, Repetition, TokenAttribute, TokenConstraint,
    TokenIndex, TokenMatcher, TokenPattern, TokenPatternItem};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load the model once. It holds everything SpaRs learned about English.
    let nlp = Model::load("assets/en_core_web_md-3.8.0")?;

    // A matcher holds named rules. Each rule is a list of patterns to look for.
    let mut matcher = TokenMatcher::new();

    // This pattern describes two tokens in a row.
    matcher.add("visit_place", vec![TokenPattern { tokens: vec![
        // First token: any form of the verb "visit". The lemma is the dictionary
        // form of a word, so "visits", "visited" and "visiting" all have the lemma "visit".
        TokenPatternItem {
            constraints: vec![TokenConstraint {
                attribute: TokenAttribute::Lemma,
                predicate: Predicate::Equals { value: "visit".into() },
            }],
            repetition: Repetition::Once,
        },
        // Then: one or more proper nouns. PROPN is the part-of-speech tag for names
        // such as "New" and "York".
        TokenPatternItem {
            constraints: vec![TokenConstraint {
                attribute: TokenAttribute::Pos,
                predicate: Predicate::Equals { value: "PROPN".into() },
            }],
            repetition: Repetition::OneOrMore,
        },
    ] }])?;

    // Read the sentence. The tokens are: Alice(0) visited(1) New(2) York(3) .(4)
    let doc = nlp.process("Alice visited New York.")?;

    // Ask the matcher for every place the pattern fits.
    let matches = matcher.find_matches(&doc)?;

    // One match starts at token 1 and stops before token 4. The end is exclusive,
    // so the match covers tokens 1, 2 and 3.
    assert!(matches.iter().any(|m| m.rule == "visit_place"
        && m.start == TokenIndex(1) && m.end == TokenIndex(4)));

    // Turn that match back into text.
    let complete = matches.iter().find(|m| m.end == TokenIndex(4)).unwrap();
    assert_eq!(doc.span_text(complete.start, complete.end)?, "visited New York");
    Ok(())
}
```

```typescript
import assert from 'node:assert/strict';
import { loadModel, TokenMatcher } from '@spars/node';

// Load the model once. It holds everything SpaRs learned about English.
const nlp = await loadModel('assets/en_core_web_md-3.8.0');

// A matcher holds named rules. Each rule is a list of patterns to look for.
const matcher = new TokenMatcher();

// This pattern describes two tokens in a row.
matcher.add('visit_place', [{ tokens: [
  // First token: any form of the verb "visit". The lemma is the dictionary
  // form of a word, so "visits", "visited" and "visiting" all have the lemma "visit".
  {
    constraints: [{ attribute: 'lemma', predicate: { kind: 'equals', value: 'visit' } }],
    repetition: { kind: 'once' },
  },
  // Then: one or more proper nouns. PROPN is the part-of-speech tag for names
  // such as "New" and "York".
  {
    constraints: [{ attribute: 'pos', predicate: { kind: 'equals', value: 'PROPN' } }],
    repetition: { kind: 'one_or_more' },
  },
] }]);

// Read the sentence. `processDocument` returns the kind of document a matcher
// searches. The tokens are: Alice(0) visited(1) New(2) York(3) .(4)
const doc = await nlp.processDocument('Alice visited New York.');

// Ask the matcher for every place the pattern fits.
const matches = await matcher.findMatches(doc);

// One match starts at token 1 and stops before token 4. The end is exclusive,
// so the match covers tokens 1, 2 and 3.
assert.ok(matches.some((match) => match.rule === 'visit_place'
  && match.start === 1 && match.end === 4));

// Turn that match back into text.
const complete = matches.find((match) => match.end === 4);
assert.ok(complete);
assert.equal(doc.span(complete.start, complete.end).text, 'visited New York');
```

:::

The default returns overlapping matches, so “visited New” also matches this pattern. A pattern describes token conditions; it does not verify that a name is a place. Use entity annotations separately when that distinction matters.

## Match without regard to case

`TokenAttribute::Lower` compares the lowercase form of each token's text, like spaCy's `LOWER`. This example finds “the Ritz” however it is capitalized.

::: code-group

```rust
use spars::{Model, Predicate, TokenAttribute, TokenConstraint, TokenIndex,
    TokenMatcher, TokenPattern, TokenPatternItem};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load the model once. It splits text into tokens (words and punctuation marks).
    let nlp = Model::load("assets/en_core_web_md-3.8.0")?;

    // A small helper that builds one pattern item. The item matches a token whose
    // lowercase text equals `value`, so "THE", "The" and "the" all match "the".
    // The default repetition is "exactly one token".
    let lower = |value: &str| TokenPatternItem {
        constraints: vec![TokenConstraint {
            attribute: TokenAttribute::Lower,
            predicate: Predicate::Equals { value: value.into() },
        }],
        repetition: Default::default(),
    };

    // A matcher holds named rules. Each rule is a list of patterns to look for.
    let mut matcher = TokenMatcher::new();

    // The rule "ritz" looks for two tokens in a row: "the", then "ritz".
    // The values are written in lowercase because they are compared as written.
    matcher.add("ritz", vec![TokenPattern { tokens: vec![lower("the"), lower("ritz")] }])?;

    // Read the sentence. A token index is a token's position, counting from 0.
    // The tokens are: THE(0) RITZ(1) is(2) near(3) the(4) Ritz(5) .(6)
    let doc = nlp.process("THE RITZ is near the Ritz.")?;

    // Find every match and keep only where each one starts and ends.
    let found: Vec<_> = matcher.find_matches(&doc)?.iter()
        .map(|m| (m.start, m.end)).collect();

    // Both spellings are found. The end is exclusive, meaning the match stops just
    // before that token: (0, 2) covers tokens 0 and 1, and (4, 6) covers tokens 4 and 5.
    assert_eq!(found, [(TokenIndex(0), TokenIndex(2)), (TokenIndex(4), TokenIndex(6))]);
    Ok(())
}
```

```typescript
import assert from 'node:assert/strict';
import { loadModel, TokenMatcher } from '@spars/node';
import type { TokenPatternItem } from '@spars/node';

// Load the model once. It splits text into tokens (words and punctuation marks).
const nlp = await loadModel('assets/en_core_web_md-3.8.0');

// A small helper that builds one pattern item. The item matches a token whose
// lowercase text equals `value`, so "THE", "The" and "the" all match "the".
// The repetition 'once' means "exactly one token".
const lower = (value: string): TokenPatternItem => ({
  constraints: [{ attribute: 'lower', predicate: { kind: 'equals', value } }],
  repetition: { kind: 'once' },
});

// A matcher holds named rules. Each rule is a list of patterns to look for.
const matcher = new TokenMatcher();

// The rule "ritz" looks for two tokens in a row: "the", then "ritz".
// The values are written in lowercase because they are compared as written.
matcher.add('ritz', [{ tokens: [lower('the'), lower('ritz')] }]);

// Read the sentence. A token index is a token's position, counting from 0.
// The tokens are: THE(0) RITZ(1) is(2) near(3) the(4) Ritz(5) .(6)
const doc = await nlp.processDocument('THE RITZ is near the Ritz.');

// Find every match and keep only where each one starts and ends.
const found = (await matcher.findMatches(doc)).map((match) => [match.start, match.end]);

// Both spellings are found. The end is exclusive, meaning the match stops just
// before that token: [0, 2] covers tokens 0 and 1, and [4, 6] covers tokens 4 and 5.
assert.deepEqual(found, [[0, 2], [4, 6]]);
```

:::

Pattern values are compared exactly as written; they are not lowercased. A value such as `"Ritz"` therefore never matches, because lowercasing always turns `R` into `r`. Write pattern values in lowercase. Lowercase mappings follow Python's Unicode 15.0.0 `str.lower()`. This includes the Greek final sigma rule: a capital `Σ` becomes `ς` only when a cased letter comes before it and no cased letter follows it, ignoring combining marks between them; otherwise it becomes `σ`. So `ΟΣ` becomes `ος`, while a lone `Σ` becomes `σ`. This is lowercasing, not case folding or Unicode normalization: `ß` does not become `ss`, and a single-character `é` stays distinct from an `e` followed by a separate combining accent. `Lower` needs only token text, not model annotations, and it differs from `Norm`, which can replace a word such as `Gon` with `going`. It uses the same pinned lowercase data as PhraseMatcher `LOWER`. `Lower` accepts `Equals`, `In`, `NotIn` and the set comparisons described in [Compare with a set of values](#compare-with-a-set-of-values); `MorphSuperset` and `MorphIntersects` apply only to `Morphology`.

## Match lexical flags

Lexical flags are true/false properties computed from a token's text, such as whether it is alphabetic or a stop word, like spaCy's `IS_ALPHA` or `LIKE_URL`. All seventeen of spaCy's lexical flag attributes are supported: `IsAlpha`, `IsDigit`, `IsSpace`, `IsPunct`, `LikeNum`, `IsLower`, `IsUpper`, `IsTitle`, `IsAscii`, `IsCurrency`, `IsStop`, `IsBracket`, `IsQuote`, `IsLeftPunct`, `IsRightPunct`, `LikeUrl` and `LikeEmail`. Some depend on the language: English `LIKE_NUM` accepts number words such as “ten” and ordinals such as “3rd”. A matcher therefore needs the model's lexicon, its language data such as Unicode character tables and number words, just as spaCy creates a matcher from the model's vocabulary. The supported models are English, and `LikeNum` implements spaCy's English rule. This example finds a number followed by a word.

::: code-group

```rust
use spars::{Model, Predicate, TokenAttribute, TokenConstraint, TokenIndex,
    TokenMatcher, TokenPattern, TokenPatternItem};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load the model once. It splits text into tokens (words and punctuation marks).
    let nlp = Model::load("assets/en_core_web_md-3.8.0")?;

    // A small helper that builds one pattern item. The item matches exactly one
    // token for which the given flag (a true/false property of its text) is true.
    let flag = |attribute| TokenPatternItem {
        constraints: vec![TokenConstraint { attribute, predicate: Predicate::Flag { value: true } }],
        repetition: Default::default(),
    };

    // Flags depend on the language. For example, English counts "ten" as a number.
    // The lexicon is the model's language data, so the matcher is built with it.
    let mut matcher = TokenMatcher::with_lexicon(nlp.lexicon());

    // The rule "quantity" looks for two tokens in a row: something that looks like
    // a number, then a token made only of letters.
    matcher.add("quantity", vec![TokenPattern {
        tokens: vec![flag(TokenAttribute::LikeNum), flag(TokenAttribute::IsAlpha)],
    }])?;

    // Read the sentence. A token index is a token's position, counting from 0.
    // The tokens are: Ten(0) people(1) paid(2) 1,000(3) dollars(4) .(5)
    let doc = nlp.process("Ten people paid 1,000 dollars.")?;

    // Find every match and keep only where each one starts and ends.
    let found: Vec<_> = matcher.find_matches(&doc)?.iter()
        .map(|m| (m.start, m.end)).collect();

    // Two matches: "Ten people" and "1,000 dollars". The end is exclusive, meaning
    // the match stops just before that token, so (0, 2) covers tokens 0 and 1.
    assert_eq!(found, [(TokenIndex(0), TokenIndex(2)), (TokenIndex(3), TokenIndex(5))]);
    Ok(())
}
```

```typescript
import assert from 'node:assert/strict';
import { loadModel, TokenMatcher } from '@spars/node';
import type { FlagAttribute, TokenPatternItem } from '@spars/node';

// Load the model once. It splits text into tokens (words and punctuation marks).
const nlp = await loadModel('assets/en_core_web_md-3.8.0');

// A small helper that builds one pattern item. The item matches exactly one
// token for which the given flag (a true/false property of its text) is true.
const flag = (attribute: FlagAttribute): TokenPatternItem => ({
  constraints: [{ attribute, predicate: { kind: 'flag', value: true } }],
  repetition: { kind: 'once' },
});

// Flags depend on the language. For example, English counts "ten" as a number.
// The model carries that language data, so the matcher is built with the model.
const matcher = new TokenMatcher(nlp);

// The rule "quantity" looks for two tokens in a row: something that looks like
// a number, then a token made only of letters.
matcher.add('quantity', [{ tokens: [flag('like_num'), flag('is_alpha')] }]);

// Read the sentence. A token index is a token's position, counting from 0.
// The tokens are: Ten(0) people(1) paid(2) 1,000(3) dollars(4) .(5)
const doc = await nlp.processDocument('Ten people paid 1,000 dollars.');

// Find every match and keep only where each one starts and ends.
const found = (await matcher.findMatches(doc)).map((match) => [match.start, match.end]);

// Two matches: "Ten people" and "1,000 dollars". The end is exclusive, meaning
// the match stops just before that token, so [0, 2] covers tokens 0 and 1.
assert.deepEqual(found, [[0, 2], [3, 5]]);
```

:::

It finds “Ten people” (tokens 0–2) and “1,000 dollars” (tokens 3–5). “Ten” is an English number word, and the comma in “1,000” is ignored. `IsAlpha` requires letters only, so “1,000” itself is not a word.

`IsAlpha`, `IsDigit` and `IsSpace` match Python's `str.isalpha()`, `str.isdigit()` and `str.isspace()`. So the superscript `²` counts as a digit but the fraction `½` does not, and the Roman numeral `Ⅻ` and a decomposed `é` (an `e` followed by a combining accent) are not alphabetic. `IsPunct` follows spaCy: every character must be in a Unicode punctuation category, so `@` and `_` are punctuation but `$` and `+` are not. English `LikeNum` removes one leading `+`, `-`, `±` or `~` and all `,` and `.` characters, then accepts digits, two digit strings joined by one `/` such as `1/2`, number and ordinal words in any capitalization such as `ten` and `Tenth`, and digits followed by `st`, `nd`, `rd` or `th`, even `3th`. It rejects `1.5e3`, `1/2/3` and hyphenated words such as `twenty-one`.

The case flags follow Python's string methods: `IsLower`, `IsUpper` and `IsTitle` need at least one cased letter, so `123` is none of them, while `A1` counts as both uppercase and titlecase and the titlecase letter `ǅ` counts as titlecase. `IsAscii` requires every character to be ASCII. `IsCurrency` requires every character to be a currency symbol, so `$`, `€` and `¥` match but `USD` and `$5` do not. `IsStop` compares the lowercase text with the language's stop words, so `the`, `The` and `THE` all match. `IsBracket`, `IsQuote`, `IsLeftPunct` and `IsRightPunct` use spaCy's fixed lists of punctuation strings, which include the two-character quotes ` `` ` (left only) and `''` (right only); the straight quotes `"` and `'` count as both left and right punctuation. `LikeUrl` follows spaCy's heuristic rather than validating a URL. Text starting with `http://` or `https://`, or with `www.` and at least one more character, always matches. Otherwise, text that starts or ends with `.` or contains `@` never matches. The rest matches when the part after its last `.` (ignoring any `:port`) is a known top-level domain or ends with `/`, or when it matches the model's URL pattern. So `example.org`, `example.com/` and even `file.txt` match, while `localhost:8080`, `www.`, `.com` and `a@b.com` do not. `LikeEmail` matches text that begins with an email address, such as `a@b.com`, but not `@handle`.

This example finds email addresses and links with two of these flags:

::: code-group

```rust
use spars::{Model, Predicate, TokenAttribute, TokenConstraint, TokenIndex,
    TokenMatcher, TokenPattern, TokenPatternItem};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load the model once. It splits text into tokens (words and punctuation marks).
    let nlp = Model::load("assets/en_core_web_md-3.8.0")?;

    // A small helper that builds a whole pattern with a single item: exactly one
    // token for which the given flag is true.
    let flag = |attribute| TokenPattern { tokens: vec![TokenPatternItem {
        constraints: vec![TokenConstraint { attribute, predicate: Predicate::Flag { value: true } }],
        repetition: Default::default(),
    }] };

    // Flag conditions need the model's lexicon (its language data), so the matcher
    // is built with it.
    let mut matcher = TokenMatcher::with_lexicon(nlp.lexicon());

    // Two rules with different names, so each match says which kind of thing it found.
    // "email" matches a token that looks like an email address.
    matcher.add("email", vec![flag(TokenAttribute::LikeEmail)])?;
    // "link" matches a token that looks like a web address (a URL).
    matcher.add("link", vec![flag(TokenAttribute::LikeUrl)])?;

    // Read the text. The address and the link each stay together as one token.
    // The tokens are: Email(0) a@b.com(1) or(2) visit(3) https://spacy.io(4)
    let doc = nlp.process("Email a@b.com or visit https://spacy.io")?;

    // Find every match and keep the rule name and the index of the token it starts at.
    let found: Vec<_> = matcher.find_matches(&doc)?.iter()
        .map(|m| (m.rule.as_str().to_owned(), m.start)).collect();

    // The "email" rule found token 1 and the "link" rule found token 4.
    assert_eq!(found, [("email".to_owned(), TokenIndex(1)), ("link".to_owned(), TokenIndex(4))]);
    Ok(())
}
```

```typescript
import assert from 'node:assert/strict';
import { loadModel, TokenMatcher } from '@spars/node';
import type { FlagAttribute, TokenPattern } from '@spars/node';

// Load the model once. It splits text into tokens (words and punctuation marks).
const nlp = await loadModel('assets/en_core_web_md-3.8.0');

// A small helper that builds a whole pattern with a single item: exactly one
// token for which the given flag is true.
const flag = (attribute: FlagAttribute): TokenPattern => ({ tokens: [{
  constraints: [{ attribute, predicate: { kind: 'flag', value: true } }],
  repetition: { kind: 'once' },
}] });

// Flag conditions need the model's language data, so the matcher is built with
// the model.
const matcher = new TokenMatcher(nlp);

// Two rules with different names, so each match says which kind of thing it found.
// "email" matches a token that looks like an email address.
matcher.add('email', [flag('like_email')]);
// "link" matches a token that looks like a web address (a URL).
matcher.add('link', [flag('like_url')]);

// Read the text. The address and the link each stay together as one token.
// The tokens are: Email(0) a@b.com(1) or(2) visit(3) https://spacy.io(4)
const doc = await nlp.processDocument('Email a@b.com or visit https://spacy.io');

// Find every match and keep the rule name and the index of the token it starts at.
const found = (await matcher.findMatches(doc)).map((match) => [match.rule, match.start]);

// The "email" rule found token 1 and the "link" rule found token 4.
assert.deepEqual(found, [['email', 1], ['link', 4]]);
```

:::

Flag attributes accept only `Predicate::Flag { value }` with `true` or `false`. In JSON the predicate is `{"kind": "flag", "value": true}`; strings, numbers and `null` are rejected. spaCy's pattern validation (`validate=True`) rejects strings and numbers for these attributes, and spaCy raises an error for `null` when the pattern is added. Flags are computed from token text, so they need no model annotations. `TokenMatcher::new()` still works for all other conditions; registering a flag condition on it returns a pattern error. The lexicon handle is cheap to clone and stays valid after the model is dropped. Token-level attributes such as `IS_SENT_START` and `SPACY` are not lexical flags and are not supported.

## Match by token length

`TokenAttribute::Length` compares the number of characters (Unicode code points) in a token, like spaCy's `LENGTH`. This example finds words of at least seven characters.

::: code-group

```rust
use spars::{Comparison, FiniteNumber, Model, Predicate, TokenAttribute, TokenConstraint,
    TokenIndex, TokenMatcher, TokenPattern, TokenPatternItem};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load the model once. It splits text into tokens (words and punctuation marks).
    let nlp = Model::load("assets/en_core_web_md-3.8.0")?;

    // Length comes from the token text alone, so a plain matcher is enough.
    let mut matcher = TokenMatcher::new();

    // The rule "long_word" has one pattern with one item: a single token whose
    // length is 7 characters or more.
    matcher.add("long_word", vec![TokenPattern { tokens: vec![TokenPatternItem {
        constraints: vec![TokenConstraint {
            attribute: TokenAttribute::Length,
            // Compare the length with a number. `FiniteNumber` refuses values that
            // are not ordinary numbers (NaN and infinity), so building one can fail.
            predicate: Predicate::Compare {
                operator: Comparison::GreaterOrEqual,
                value: FiniteNumber::new(7.0).ok_or("not finite")?,
            },
        }],
        repetition: Default::default(),
    }] }])?;

    // Read the sentence. A token index is a token's position, counting from 0.
    // The tokens are: Internationalization(0) is(1) complicated(2) .(3)
    let doc = nlp.process("Internationalization is complicated.")?;

    // Find every match and keep only where each one starts and ends.
    let found: Vec<_> = matcher.find_matches(&doc)?.iter()
        .map(|m| (m.start, m.end)).collect();

    // Tokens 0 and 2 are long enough; "is" and "." are too short. The end is
    // exclusive, meaning the match stops just before it, so (0, 1) is token 0 alone.
    assert_eq!(found, [(TokenIndex(0), TokenIndex(1)), (TokenIndex(2), TokenIndex(3))]);
    Ok(())
}
```

```typescript
import assert from 'node:assert/strict';
import { loadModel, TokenMatcher } from '@spars/node';

// Load the model once. It splits text into tokens (words and punctuation marks).
const nlp = await loadModel('assets/en_core_web_md-3.8.0');

// Length comes from the token text alone, so a plain matcher is enough.
const matcher = new TokenMatcher();

// The rule "long_word" has one pattern with one item: a single token whose
// length is 7 characters or more.
matcher.add('long_word', [{ tokens: [{
  constraints: [{
    attribute: 'length',
    // Compare the length with a number. The value must be an ordinary number:
    // `add` throws an error for NaN (not a number) and for infinity.
    predicate: { kind: 'compare', operator: '>=', value: 7 },
  }],
  repetition: { kind: 'once' },
}] }]);

// Read the sentence. A token index is a token's position, counting from 0.
// The tokens are: Internationalization(0) is(1) complicated(2) .(3)
const doc = await nlp.processDocument('Internationalization is complicated.');

// Find every match and keep only where each one starts and ends.
const found = (await matcher.findMatches(doc)).map((match) => [match.start, match.end]);

// Tokens 0 and 2 are long enough; "is" and "." are too short. The end is
// exclusive, meaning the match stops just before it, so [0, 1] is token 0 alone.
assert.deepEqual(found, [[0, 1], [2, 3]]);
```

:::

It finds “Internationalization” (20 characters) and “complicated” (11), but not “is” or the final period.

Length counts Unicode code points, like Python's `len()`, not bytes or user-perceived characters. A single-character `é` has length 1, but an `e` followed by a combining accent has length 2, and the emoji `👩🏽‍💻` has length 4 because it combines four code points. `Length` needs only the token text, so it works without model annotations or a lexicon.

`Length` accepts these predicates, plus the integer set predicates described in [Compare with a set of values](#compare-with-a-set-of-values):

| Predicate | JSON | spaCy form |
|---|---|---|
| `Compare { operator, value }` | `{"kind": "compare", "operator": ">=", "value": 7}` | `{"LENGTH": {">=": 7}}` |
| `InIntegers { values }` | `{"kind": "in_integers", "values": [1, 3]}` | `{"LENGTH": {"IN": [1, 3]}}` |
| `NotInIntegers { values }` | `{"kind": "not_in_integers", "values": [1, 3]}` | `{"LENGTH": {"NOT_IN": [1, 3]}}` |

The operators are `==`, `!=`, `>=`, `<=`, `>` and `<`. The value may be a whole or fractional number, so `> 2.5` matches lengths of 3 and more, as in spaCy. `Compare` corresponds to spaCy's dictionary form, which also accepts negative and fractional values. SpaRs has no shorthand for spaCy's exact form `{"LENGTH": 3}`; write it as `Compare` with `==`, for example `{"kind": "compare", "operator": "==", "value": 3}`. Several conditions on one item must all match, so `>= 2` together with `< 4` selects lengths 2 and 3. `FiniteNumber` rejects NaN and infinities; spaCy accepts them in Python patterns, where NaN matches nothing and infinities compare as expected, but JSON patterns cannot express them. `InIntegers` and `NotInIntegers` take whole numbers; as with spaCy's pattern validation, fractional values such as `3.0` are rejected. Values must fit in a 64-bit signed integer (`i64`). spaCy accepts larger Python integers, which never equal a token length, but SpaRs rejects them with a pattern error, so a spaCy `NOT_IN` pattern containing such a value (which matches every token) cannot be written here. Node accepts only safe JavaScript integers, up to 2^53−1 in magnitude. Strings, booleans and `null` are rejected, and other predicates on `Length` return a pattern error. `Length` also accepts the integer set predicates described in [Compare with a set of values](#compare-with-a-set-of-values). spaCy's `REGEX` on `LENGTH` is not supported.

## Compare with a set of values

spaCy's `IS_SUBSET`, `IS_SUPERSET` and `INTERSECTS` compare a token with a list of values. SpaRs supports them on every string attribute (`Text`, `Lower`, `Norm`, `Lemma`, `Pos`, `Tag`, `Dep` and `Morphology`) and on `Length`. Annotation attributes such as `Morphology` read the document's annotations, so use a document processed by the model. For tokens without the annotation, see [Missing annotations](#missing-annotations). `Length` needs only the token text. This example finds plural words of three or four characters.

::: code-group

```rust
use spars::{Model, Predicate, TokenAttribute, TokenConstraint, TokenIndex, TokenMatcher,
    TokenPattern, TokenPatternItem};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load the model once. It splits text into tokens and works out the grammar
    // of each one.
    let nlp = Model::load("assets/en_core_web_md-3.8.0")?;

    // A matcher holds named rules. Each rule is a list of patterns to look for.
    let mut matcher = TokenMatcher::new();

    // The rule "short_plural" has one pattern with one item. The item has two
    // conditions, and a token must pass both of them.
    matcher.add("short_plural", vec![TokenPattern { tokens: vec![TokenPatternItem {
        constraints: vec![
            // Condition 1: the token is plural. Morphology is the list of grammar
            // features of a word, such as "Number=Plur" for a plural. `IsSuperset`
            // passes when the token has every feature listed here.
            TokenConstraint {
                attribute: TokenAttribute::Morphology,
                predicate: Predicate::IsSuperset { values: vec!["Number=Plur".into()] },
            },
            // Condition 2: the token is 3 or 4 characters long. `IsSubsetIntegers`
            // passes when the token's length is one of the listed numbers.
            TokenConstraint {
                attribute: TokenAttribute::Length,
                predicate: Predicate::IsSubsetIntegers { values: vec![3, 4] },
            },
        ],
        repetition: Default::default(),
    }] }])?;

    // Read the sentence. A token index is a token's position, counting from 0.
    // The tokens are: Two(0) cats(1) and(2) three(3) dogs(4) were(5) here(6) .(7)
    let doc = nlp.process("Two cats and three dogs were here.")?;

    // Find every match and keep only where each one starts and ends.
    let found: Vec<_> = matcher.find_matches(&doc)?.iter()
        .map(|m| (m.start, m.end)).collect();

    // Only "cats" (token 1) and "dogs" (token 4) are both plural and short. The end
    // is exclusive, meaning the match stops just before it, so (1, 2) is token 1 alone.
    assert_eq!(found, [(TokenIndex(1), TokenIndex(2)), (TokenIndex(4), TokenIndex(5))]);
    Ok(())
}
```

```typescript
import assert from 'node:assert/strict';
import { loadModel, TokenMatcher } from '@spars/node';

// Load the model once. It splits text into tokens and works out the grammar
// of each one.
const nlp = await loadModel('assets/en_core_web_md-3.8.0');

// A matcher holds named rules. Each rule is a list of patterns to look for.
const matcher = new TokenMatcher();

// The rule "short_plural" has one pattern with one item. The item has two
// conditions, and a token must pass both of them.
matcher.add('short_plural', [{ tokens: [{
  constraints: [
    // Condition 1: the token is plural. Morphology is the list of grammar
    // features of a word, such as "Number=Plur" for a plural. `is_superset`
    // passes when the token has every feature listed here.
    { attribute: 'morphology', predicate: { kind: 'is_superset', values: ['Number=Plur'] } },
    // Condition 2: the token is 3 or 4 characters long. `is_subset_integers`
    // passes when the token's length is one of the listed numbers.
    { attribute: 'length', predicate: { kind: 'is_subset_integers', values: [3, 4] } },
  ],
  repetition: { kind: 'once' },
}] }]);

// Read the sentence. A token index is a token's position, counting from 0.
// The tokens are: Two(0) cats(1) and(2) three(3) dogs(4) were(5) here(6) .(7)
const doc = await nlp.processDocument('Two cats and three dogs were here.');

// Find every match and keep only where each one starts and ends.
const found = (await matcher.findMatches(doc)).map((match) => [match.start, match.end]);

// Only "cats" (token 1) and "dogs" (token 4) are both plural and short. The end
// is exclusive, meaning the match stops just before it, so [1, 2] is token 1 alone.
assert.deepEqual(found, [[1, 2], [4, 5]]);
```

:::

It finds “cats” and “dogs”, whose morphology includes `Number=Plur`, but not the numbers or “were”.

As in spaCy, a token's value is treated as a set with one element, except morphology, which is the set of its individual features such as `Number=Plur` and `Tense=Past`:

| Predicate | JSON | spaCy form | Holds when |
|---|---|---|---|
| `IsSubset { values }` | `{"kind": "is_subset", "values": ["a", "b"]}` | `{"LOWER": {"IS_SUBSET": ["a", "b"]}}` | the token's value is listed (morphology: every feature of the token is listed) |
| `IsSuperset { values }` | `{"kind": "is_superset", "values": ["a"]}` | `{"LOWER": {"IS_SUPERSET": ["a"]}}` | every listed value equals the token's value (morphology: every listed value is a feature of the token) |
| `Intersects { values }` | `{"kind": "intersects", "values": ["a", "b"]}` | `{"LOWER": {"INTERSECTS": ["a", "b"]}}` | at least one listed value equals the token's value (morphology: is a feature of the token) |
| `IsSubsetIntegers { values }`, `IsSupersetIntegers { values }`, `IntersectsIntegers { values }` | `{"kind": "is_subset_integers", "values": [3, 4]}` | `{"LENGTH": {"IS_SUBSET": [3, 4]}}` | the same rules for `Length` |

So on a single-valued attribute, `IsSubset` and `Intersects` behave like `In`, and `IsSuperset` holds when the list is empty or contains only the token's value. An empty list makes `Intersects` fail and `IsSuperset` succeed. It makes `IsSubset` fail too, except on morphology for a token with no features, because an empty analysis is a subset of every list. This matches spaCy. Listed morphology entries are normalized like the other morphology predicates. An entry with several values or fields, such as `Case=Acc,Nom` or `Number=Sing|Person=3`, never equals a single feature, and neither do `_` or the empty string; such entries never satisfy `IsSuperset` or `Intersects` and add nothing to `IsSubset`. The older `MorphSuperset` and `MorphIntersects` remain valid on morphology and behave like `IsSuperset` and `Intersects`.

String sets take strings and integer sets take whole numbers, as spaCy's pattern validation requires; SpaRs also limits integers to `i64` (see [Match by token length](#match-by-token-length)). Set predicates on lexical flags, string sets on `Length` and integer sets on string attributes return pattern errors. Apart from morphology normalization, values are compared as written; `Lower` values are not lowercased.

## Keep matches that do not overlap

By default a rule reports every match, including matches that share tokens. spaCy's `Matcher.add` has a `greedy` argument that makes a rule report only matches that share no token with each other, and SpaRs has the same option with the same two values. Both work the same way: the rule's matches are put in an order of preference, and each one in turn is kept unless it shares a token with a match already kept. It is called greedy because a kept match is never given up for a later one. `Greedy::First` orders matches by start, earliest first, and puts the longer match first when two start at the same token. `Greedy::Longest` orders them by length, longest first, and puts the earlier match first when two are equally long. In Rust, pass the option with `add_with`; in Node.js, pass it as the third argument of `add`. This example registers the same two patterns three times: without the option, with `First` and with `Longest`.

::: code-group

```rust
use spars::{Greedy, Model, Predicate, TokenAttribute, TokenConstraint, TokenMatcher,
    TokenPattern, TokenPatternItem, TokenRuleOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load the model once. Here it only splits the text into tokens.
    let nlp = Model::load("assets/en_core_web_md-3.8.0")?;

    // A pattern that matches these exact words, one after another.
    let phrase = |words: &[&str]| TokenPattern {
        tokens: words.iter().map(|word| TokenPatternItem {
            constraints: vec![TokenConstraint {
                attribute: TokenAttribute::Text,
                predicate: Predicate::Equals { value: (*word).into() },
            }],
            repetition: Default::default(),
        }).collect(),
    };
    // Two patterns that overlap on the word "York".
    let places = || vec![phrase(&["New", "York"]), phrase(&["York", "City", "Hall"])];

    let mut matcher = TokenMatcher::new();

    // A rule added with `add` reports every match.
    matcher.add("every", places())?;

    // `add_with` takes options for the rule. Start from the defaults and set `greedy`.
    let mut first = TokenRuleOptions::default();
    first.greedy = Some(Greedy::First);
    matcher.add_with("first", places(), first)?;

    let mut longest = TokenRuleOptions::default();
    longest.greedy = Some(Greedy::Longest);
    matcher.add_with("longest", places(), longest)?;

    // The tokens are: They(0) met(1) at(2) New(3) York(4) City(5) Hall(6) .(7)
    let doc = nlp.process("They met at New York City Hall.")?;

    let found: Vec<_> = matcher.find_matches(&doc)?.into_iter()
        .map(|m| (m.rule, m.start.0, m.end.0)).collect();

    // "New York" is tokens 3 and 4; "York City Hall" is tokens 4, 5 and 6. They
    // share token 4. The rule "first" keeps the one that starts earlier, and the
    // rule "longest" keeps the longer one.
    assert_eq!(found, [
        ("every".to_string(), 3, 5),
        ("every".to_string(), 4, 7),
        ("first".to_string(), 3, 5),
        ("longest".to_string(), 4, 7),
    ]);
    Ok(())
}
```

```typescript
import assert from 'node:assert/strict';
import { loadModel, TokenMatcher } from '@spars/node';
import type { TokenPattern } from '@spars/node';

// Load the model once. Here it only splits the text into tokens.
const nlp = await loadModel('assets/en_core_web_md-3.8.0');

// A pattern that matches these exact words, one after another.
const phrase = (words: string[]): TokenPattern => ({
  tokens: words.map((word) => ({
    constraints: [{ attribute: 'text', predicate: { kind: 'equals', value: word } }],
    repetition: { kind: 'once' },
  })),
});
// Two patterns that overlap on the word "York".
const places = [phrase(['New', 'York']), phrase(['York', 'City', 'Hall'])];

const matcher = new TokenMatcher();

// Without options, `add` reports every match of the rule.
matcher.add('every', places);

// The third argument holds options for the rule, like the keyword arguments
// of spaCy's `Matcher.add`.
matcher.add('first', places, { greedy: 'FIRST' });
matcher.add('longest', places, { greedy: 'LONGEST' });

// The tokens are: They(0) met(1) at(2) New(3) York(4) City(5) Hall(6) .(7)
const doc = await nlp.processDocument('They met at New York City Hall.');

const found = (await matcher.findMatches(doc)).map((match) => [match.rule, match.start, match.end]);

// "New York" is tokens 3 and 4; "York City Hall" is tokens 4, 5 and 6. They
// share token 4. The rule "first" keeps the one that starts earlier, and the
// rule "longest" keeps the longer one.
assert.deepEqual(found, [
  ['every', 3, 5],
  ['every', 4, 7],
  ['first', 3, 5],
  ['longest', 4, 7],
]);
```

:::

The rule “every” reports both “New York” and “York City Hall”, which share the token “York”. The rule “first” keeps only “New York”, because it starts earlier, and the rule “longest” keeps only “York City Hall”, because it is longer.

The option belongs to the rule, not to one pattern or one search, and it follows spaCy in three ways that are easy to miss:

- **The latest registration decides.** Each `add` or `add_with` call sets the options for the whole rule, so they also apply to the patterns that earlier calls added. Calling plain `add` on a rule that has the option removes it, because `add` is `add_with` with the default options; this is what spaCy does when `Matcher.add` is called without `greedy`. On an existing rule, a call with an empty list of patterns changes only the options; with a new name, it registers a rule that has options and no patterns. A call that returns an error leaves the rule's patterns and options as they were; spaCy likewise keeps the earlier `greedy` value.
- **Rules do not compete with each other.** The option removes overlaps among the matches of one rule. Two rules can still report matches that share tokens, as “first” and “longest” do above.
- **Greedy rules come last in the result.** `find_matches` (`findMatches` in Node.js) first lists the matches of rules without the option, in the usual [discovery order](#reuse-ordering-and-errors). Then it lists the kept matches of each rule with the option, in the order in which those rules first matched, and within a rule the most preferred match first. With `Longest`, a long match late in the document comes before a shorter one earlier in it. Sort the result yourself if you need document order.

Repetition is unchanged: `OneOrMore` and `ZeroOrMore` still produce every possible match, and the option then chooses among them. `Longest` on a `OneOrMore` item is the usual way to get each whole run of tokens once instead of every part of it. The option filters after matching: every match is still found first, so it reduces the result, not the matching time described in [Scope and cost](#scope-and-cost).

| Rust | spaCy | Node.js | Keeps |
|---|---|---|---|
| `None` (default) | `None` or omitted | omitted or `null` | every match |
| `Some(Greedy::First)` | `"FIRST"` | `'FIRST'` | the earliest start first, then the longer match |
| `Some(Greedy::Longest)` | `"LONGEST"` | `'LONGEST'` | the longest match first, then the earliest start |

## Conditions and repetition

All conditions on an item must match the same token. Empty conditions match any token. Token Matcher shares `TokenConstraint`, `TokenAttribute`, and `Predicate` with [DependencyMatcher](DEPENDENCY_MATCHER.md), including its morphology rules. Available attributes are `Text`, `Lower` (lowercase text), `Norm`, `Lemma`, `Pos`, `Tag`, `Dep`, `Morphology`, the seventeen lexical flags, and `Length`. Comparisons support equality, membership, exclusion, the set predicates subset, superset and intersection, true/false flags, and numeric comparisons, integer membership and integer set predicates for `Length`. Text equality is case-sensitive; to ignore capitalization in the document, use `Lower` with lowercase pattern values.

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

`add` validates all supplied patterns before changing the matcher. Adding an existing rule appends its patterns. `add_with` does the same with the options you pass, and plain `add` returns the rule to the default options; see [Keep matches that do not overlap](#keep-matches-that-do-not-overlap). `get`, `contains`, `remove`, `len`, and `is_empty` manage rules. Calls use separate matching state, so registered matchers can be shared for concurrent read-only use. Matching does not invoke inference or access the network.

Results preserve the pinned spaCy matcher's discovery order, including optional suffixes at the end of the document. They are not sorted by start position or grouped by rule, except that each rule registered with `greedy` has its matches listed together, after the matches of every rule without it. Identical `(rule, start, end)` results appear once, even when several patterns or repetition paths produce them. Different rule names can match the same span.

Items with an exact zero count are discarded and do not require annotations. Text, lowercase, norm, lexical flag and length conditions do not require dependency heads or linguistic annotations. Invalid patterns and unsupported serialized enum values return errors.

### Missing annotations

On a nonempty document, an `Equals` condition on `Lemma`, `Pos` (the coarse part-of-speech tag), `Tag` (the fine-grained tag), `Dep` or `Morphology` needs at least one token with that annotation; otherwise matching returns `Error::MissingAnnotation`, even when another condition would have ruled out every token. An empty document never returns this error. When several such attributes are missing, the error names the first in spaCy's order: tag, part-of-speech tag, morphology, lemma, dependency label. This follows spaCy's error E155, which checks `Doc.has_annotation` and so accepts a partly annotated document, such as one restored from a snapshot in which only some tokens have lemmas. An empty lemma, part-of-speech tag, fine-grained tag or dependency label counts as missing, as in spaCy, while the empty morphology analysis counts as present. Membership (`In`, `NotIn`) and set conditions never require the annotation, as in spaCy.

A token without the annotation compares as the empty string `""`, so `Equals` with `""`, an `In` or set list that contains `""`, a `NotIn` list without `""`, and an empty `IsSuperset` match it. Missing morphology is spaCy's unset analysis, which differs from the known-empty analysis `_`: `Equals` matches it only with `""` (never `"_"`), `In` never matches it and `NotIn` always does, even `NotIn [""]`, and as a feature set it is empty, so it satisfies `IsSubset` and an empty `IsSuperset` but never `Intersects`.

spaCy keeps requiring an annotation that an `Equals` condition compared even after the rule containing that condition is removed. SpaRs checks only the current rules; this is an [accepted difference](SPACY_DIFFERENCES.md#annotations-required-by-removed-rules).

## Scope and cost

This API reports every match by default, overlapping ones included, and can keep only non-overlapping matches of a rule with greedy `FIRST` or `LONGEST` selection. Alignments, callbacks, regex and fuzzy predicates, custom extensions, token-level attributes such as `IS_SENT_START`, and span input are not exposed. It is a typed Rust API, not an importer for arbitrary spaCy pattern JSON. [PhraseMatcher](PHRASE_MATCHER.md) matches token phrases by text, lowercase, norm, annotations, lexical flags or length.

Patterns are limited to 4,096 expanded nodes to keep registration bounded. `OneOrMore` uses two nodes; a bounded range uses its maximum count, and an unbounded range uses its minimum plus one. Larger patterns return an error.

spaCy's matcher tries every pattern at every token. SpaRs does not try a pattern at a token that cannot satisfy its first item, when that item consumes exactly one token (`Once`, `OneOrMore`, or a `Range` with a minimum of at least one) and has an `Equals`, `In`, `IsSubset` or `Intersects` condition, or an `IsSuperset` condition with one value, on `Text`, `Lower` or `Norm`. Items with an exact zero count are ignored when finding the first item. Other patterns, such as those starting with `Optional`, `ZeroOrMore`, `Negated` or a `Range` from zero, or whose first item has only `NotIn`, annotation, lexical flag or `Length` conditions, are tried at every token, so their matching time grows with the document length multiplied by the number of such patterns. Results, their order and error reporting are the same as trying every pattern at every token; missing annotations are still reported for every registered condition. `add` and `remove` keep the value lookup up to date. `remove` renumbers every remaining pattern, so its cost grows with the number of registered patterns and of distinct values in their first items.

Broad repetition can produce a quadratic number of spans. Several optional items can also reach the same state by different paths. Prefer specific conditions and bounded repetition when they express the intended pattern. Performance measurements must include both the input length and the number of returned matches. Lexical flags are computed once per call for each token, and only for the flags that registered patterns use; spaCy instead stores them on each vocabulary entry, so repeated calls in SpaRs repeat this work.
