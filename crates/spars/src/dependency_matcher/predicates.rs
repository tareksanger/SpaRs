use crate::{Doc, Error, Lexicon, Result, TokenIndex, TokenView};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::{BTreeMap, HashSet};

/// Token annotations shared by token and dependency patterns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenAttribute {
    Text,
    /// Python Unicode 15 `str.lower()` of the token text, as in spaCy's LOWER.
    /// Pattern values are compared as written and are not lowercased.
    Lower,
    Norm,
    Lemma,
    Pos,
    Tag,
    Dep,
    Morphology,
    /// Lexical flags from the matcher's [`Lexicon`], compared with [`Predicate::Flag`].
    /// Like spaCy's `IS_ALPHA`, `IS_DIGIT`, `IS_SPACE`, `IS_PUNCT` and `LIKE_NUM`.
    IsAlpha,
    IsDigit,
    IsSpace,
    IsPunct,
    LikeNum,
}

impl TokenAttribute {
    /// The bit for a lexical flag attribute in per-token flag masks.
    fn flag_bit(self) -> Option<u8> {
        match self {
            TokenAttribute::IsAlpha => Some(1),
            TokenAttribute::IsDigit => Some(1 << 1),
            TokenAttribute::IsSpace => Some(1 << 2),
            TokenAttribute::IsPunct => Some(1 << 3),
            TokenAttribute::LikeNum => Some(1 << 4),
            _ => None,
        }
    }
    /// Whether this attribute needs a matcher created with a [`Lexicon`].
    pub(crate) fn needs_lexicon(self) -> bool {
        self.flag_bit().is_some()
    }
}

/// A comparison applied to one token attribute. Set predicates apply to morphology;
/// `Flag` applies only to lexical flag attributes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Predicate {
    Equals { value: String },
    In { values: Vec<String> },
    NotIn { values: Vec<String> },
    MorphSuperset { values: Vec<String> },
    MorphIntersects { values: Vec<String> },
    Flag { value: bool },
}

/// All constraints on a pattern node must match the same token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TokenConstraint {
    pub attribute: TokenAttribute,
    pub predicate: Predicate,
}

#[derive(Debug, Clone)]
enum CompiledPredicate {
    Equals(String),
    In(HashSet<String>),
    NotIn(HashSet<String>),
    Superset(HashSet<String>),
    Intersects(HashSet<String>),
}
#[derive(Debug, Clone)]
pub(crate) struct CompiledConstraint {
    attribute: TokenAttribute,
    predicate: CompiledPredicate,
}

/// The lexical flag conditions of one pattern item or node, compiled into a single
/// mask comparison so string predicates keep their own evaluation path.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct FlagTest {
    mask: u8,
    expected: u8,
    // Requiring a flag to be both true and false can never match.
    contradictory: bool,
}

impl FlagTest {
    fn require(&mut self, bit: u8, value: bool) {
        if self.mask & bit != 0 && (self.expected & bit != 0) != value {
            self.contradictory = true;
        }
        self.mask |= bit;
        if value {
            self.expected |= bit;
        }
    }
    pub(crate) fn mask(self) -> u8 {
        self.mask
    }
    pub(crate) fn matches(self, values: &TokenValues<'_>, index: usize) -> Result<bool> {
        if self.mask == 0 {
            return Ok(true);
        }
        let flags = values.flags.get(index).ok_or_else(missing_lexicon)?;
        Ok(!self.contradictory && flags & self.mask == self.expected)
    }
}

/// The compiled conditions shared by one pattern item or dependency node.
pub(crate) struct CompiledConditions {
    pub(crate) constraints: Vec<CompiledConstraint>,
    pub(crate) flags: FlagTest,
}

pub(crate) fn compile_conditions(constraints: &[TokenConstraint]) -> Result<CompiledConditions> {
    let mut compiled = Vec::new();
    let mut flags = FlagTest::default();
    for constraint in constraints {
        match (constraint.attribute.flag_bit(), &constraint.predicate) {
            (Some(bit), Predicate::Flag { value }) => flags.require(bit, *value),
            _ => compiled.push(constraint.compile()?),
        }
    }
    Ok(CompiledConditions {
        constraints: compiled,
        flags,
    })
}

#[cold]
fn missing_lexicon() -> Error {
    Error::Pattern("lexical flag conditions require a matcher lexicon".into())
}

/// Token values derived once per matching call and shared by every constraint.
/// Lowercase text and lexical flags are computed only for attributes in use.
pub(crate) struct TokenValues<'a> {
    lower: Vec<Cow<'a, str>>,
    flags: Vec<u8>,
}

impl<'a> TokenValues<'a> {
    pub(crate) fn new(
        doc: &'a Doc,
        attributes: &HashSet<TokenAttribute>,
        mask: u8,
        lexicon: Option<&Lexicon>,
    ) -> Result<Self> {
        let texts = || (0..doc.tokens().len()).map(|index| doc.token_text(TokenIndex(index)));
        let lower = if attributes.contains(&TokenAttribute::Lower) {
            texts().map(|text| text.map(lower)).collect::<Result<_>>()?
        } else {
            Vec::new()
        };
        let flags = if mask == 0 {
            Vec::new()
        } else {
            let lexicon = lexicon.ok_or_else(missing_lexicon)?;
            let r = lexicon.resources();
            texts()
                .map(|text| {
                    let text = text?;
                    Ok(FLAG_ATTRIBUTES.iter().fold(0, |flags, &attribute| {
                        let bit = attribute.flag_bit().unwrap_or(0);
                        if mask & bit != 0 && lexical_flag(r, attribute, text) {
                            flags | bit
                        } else {
                            flags
                        }
                    }))
                })
                .collect::<Result<_>>()?
        };
        Ok(Self { lower, flags })
    }
}

const FLAG_ATTRIBUTES: [TokenAttribute; 5] = [
    TokenAttribute::IsAlpha,
    TokenAttribute::IsDigit,
    TokenAttribute::IsSpace,
    TokenAttribute::IsPunct,
    TokenAttribute::LikeNum,
];

// The same functions compute these fields in `Model::lexeme`.
fn lexical_flag(r: &crate::config::Lexical, attribute: TokenAttribute, text: &str) -> bool {
    match attribute {
        TokenAttribute::IsAlpha => r.is_alpha(text),
        TokenAttribute::IsDigit => r.is_digit(text),
        TokenAttribute::IsSpace => r.is_space(text),
        TokenAttribute::IsPunct => r.is_punct(text),
        TokenAttribute::LikeNum => r.like_num(text),
        _ => false,
    }
}

impl TokenConstraint {
    pub(crate) fn compile(&self) -> Result<CompiledConstraint> {
        let is_morph = self.attribute == TokenAttribute::Morphology;
        let set = |values: &[String]| -> Result<HashSet<String>> {
            values
                .iter()
                .map(|value| {
                    if is_morph {
                        normalize_morph(value)
                    } else {
                        Ok(value.clone())
                    }
                })
                .collect()
        };
        let mismatched_flag = || {
            Error::Pattern(
                "lexical flag attributes require flag predicates, and flag predicates require lexical flag attributes".into(),
            )
        };
        // Valid flag conditions compile into a `FlagTest` in `compile_conditions`.
        if self.attribute.needs_lexicon() {
            return Err(mismatched_flag());
        }
        let predicate = match &self.predicate {
            Predicate::Flag { .. } => return Err(mismatched_flag()),
            Predicate::Equals { value } => CompiledPredicate::Equals(value.clone()),
            Predicate::In { values } => CompiledPredicate::In(set(values)?),
            Predicate::NotIn { values } => CompiledPredicate::NotIn(set(values)?),
            Predicate::MorphSuperset { values } | Predicate::MorphIntersects { values } => {
                if !is_morph {
                    return Err(Error::Pattern(
                        "morphology set predicates require the morphology attribute".into(),
                    ));
                }
                if matches!(self.predicate, Predicate::MorphSuperset { .. }) {
                    CompiledPredicate::Superset(set(values)?)
                } else {
                    CompiledPredicate::Intersects(set(values)?)
                }
            }
        };
        Ok(CompiledConstraint {
            attribute: self.attribute,
            predicate,
        })
    }
}

impl CompiledConstraint {
    pub(crate) fn attribute(&self) -> TokenAttribute {
        self.attribute
    }
    pub(crate) fn validate_document(&self, doc: &Doc) -> Result<()> {
        // Text-derived attributes are always available and need no document scan.
        if matches!(
            self.attribute,
            TokenAttribute::Text | TokenAttribute::Lower | TokenAttribute::Norm
        ) || self.attribute.needs_lexicon()
        {
            return Ok(());
        }
        // Check presence directly; the per-check value lookup is not needed here.
        let (field, name): (fn(&crate::Token) -> Option<&str>, _) = match self.attribute {
            TokenAttribute::Lemma => (|t| t.lemma.as_deref(), "lemma"),
            TokenAttribute::Pos => (|t| t.pos.as_deref(), "POS"),
            TokenAttribute::Tag => (|t| t.tag.as_deref(), "tag"),
            TokenAttribute::Dep => (|t| t.dep.as_deref(), "dependency label"),
            TokenAttribute::Morphology => (|t| t.morphology.as_deref(), "morphology"),
            _ => return Ok(()),
        };
        for token in doc.tokens() {
            let value = field(token).ok_or(Error::MissingAnnotation(name))?;
            if self.attribute == TokenAttribute::Morphology {
                validate_canonical_morph(value)?;
            }
        }
        Ok(())
    }
    fn value<'a>(
        &self,
        token: TokenView<'a>,
        values: Option<&'a TokenValues<'a>>,
    ) -> Result<Cow<'a, str>> {
        let t = token.annotations();
        let (value, name) = match self.attribute {
            TokenAttribute::Text => return Ok(Cow::Borrowed(token.text())),
            TokenAttribute::Lower => {
                let cached = values.and_then(|values| values.lower.get(token.index().0));
                return Ok(match cached {
                    Some(value) => Cow::Borrowed(value.as_ref()),
                    None => lower(token.text()),
                });
            }
            TokenAttribute::Norm => return Ok(Cow::Borrowed(&t.norm)),
            TokenAttribute::Lemma => (t.lemma.as_deref(), "lemma"),
            TokenAttribute::Pos => (t.pos.as_deref(), "POS"),
            TokenAttribute::Tag => (t.tag.as_deref(), "tag"),
            TokenAttribute::Dep => (t.dep.as_deref(), "dependency label"),
            TokenAttribute::Morphology => (t.morphology.as_deref(), "morphology"),
            TokenAttribute::IsAlpha
            | TokenAttribute::IsDigit
            | TokenAttribute::IsSpace
            | TokenAttribute::IsPunct
            | TokenAttribute::LikeNum => return Err(no_string_value()),
        };
        value
            .map(Cow::Borrowed)
            .ok_or(Error::MissingAnnotation(name))
    }
    pub(crate) fn matches<'a>(
        &self,
        token: TokenView<'a>,
        values: &'a TokenValues<'a>,
    ) -> Result<bool> {
        let value = self.value(token, Some(values))?;
        let value = value.as_ref();
        let feature = |wanted: &String| -> bool {
            let Some((key, val)) = wanted.split_once('=') else {
                return false;
            };
            value.split('|').any(|field| {
                field
                    .split_once('=')
                    .is_some_and(|(k, values)| k == key && values.split(',').any(|v| v == val))
            })
        };
        Ok(match &self.predicate {
            CompiledPredicate::Equals(expected) => {
                let actual = if self.attribute == TokenAttribute::Morphology && value.is_empty() {
                    "_"
                } else {
                    value
                };
                actual == expected
            }
            CompiledPredicate::In(expected) => expected.contains(value),
            CompiledPredicate::NotIn(expected) => !expected.contains(value),
            CompiledPredicate::Superset(expected) => expected.iter().all(feature),
            CompiledPredicate::Intersects(expected) => expected.iter().any(feature),
        })
    }
}

#[cold]
fn no_string_value() -> Error {
    Error::Pattern("lexical flags have no string value".into())
}

// Most tokens are lowercase ASCII already; avoid allocating for them.
fn lower(text: &str) -> Cow<'_, str> {
    if text.is_ascii() && !text.bytes().any(|byte| byte.is_ascii_uppercase()) {
        Cow::Borrowed(text)
    } else {
        Cow::Owned(crate::unicode_lower::lower(text))
    }
}

// spaCy's set predicates normalize complete morphology strings; direct equality
// instead compares the supplied string. Feature-set matching uses atomic values.
fn normalize_morph(value: &str) -> Result<String> {
    if value.is_empty() || value == "_" {
        return Ok(String::new());
    }
    let mut raw_fields: Vec<(&str, Vec<String>)> = Vec::new();
    for field in value.split('|') {
        let (key, values) = field
            .split_once('=')
            .ok_or_else(|| Error::Pattern("morphology entries must use Field=Value".into()))?;
        if key.is_empty()
            || values.is_empty()
            || field.chars().any(char::is_whitespace)
            || values.contains('=')
        {
            return Err(Error::Pattern("malformed morphology field".into()));
        }
        let mut items = Vec::new();
        for val in values.split(',') {
            if val.is_empty() {
                return Err(Error::Pattern("empty morphology value".into()));
            }
            items.push(val.to_owned());
        }
        items.sort_unstable();
        if let Some((_, existing)) = raw_fields
            .iter_mut()
            .find(|(existing_key, _)| *existing_key == key)
        {
            *existing = items;
        } else {
            raw_fields.push((key, items));
        }
    }
    // Preserve first field insertion order while replacing duplicate fields,
    // before normalizing aliases such as POS. This matches Python dict updates.
    let mut fields: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (key, mut items) in raw_fields {
        let key = if key.eq_ignore_ascii_case("POS") {
            if items.len() == 1 && known_pos(&items[0].to_ascii_uppercase()) {
                items[0] = items[0].to_ascii_uppercase();
            }
            "POS"
        } else {
            key
        };
        fields.insert(key.to_owned(), items);
    }
    Ok(fields
        .into_iter()
        .map(|(key, values)| format!("{key}={}", values.into_iter().collect::<Vec<_>>().join(",")))
        .collect::<Vec<_>>()
        .join("|"))
}

fn known_pos(value: &str) -> bool {
    matches!(
        value,
        "ADJ"
            | "ADP"
            | "ADV"
            | "AUX"
            | "CONJ"
            | "CCONJ"
            | "DET"
            | "INTJ"
            | "NOUN"
            | "NUM"
            | "PART"
            | "PRON"
            | "PROPN"
            | "PUNCT"
            | "SCONJ"
            | "SYM"
            | "VERB"
            | "X"
            | "EOL"
            | "SPACE"
    )
}

// Check normal document morphology without allocating normalization tables for
// every token. Pattern-set normalization above happens only during registration.
fn validate_canonical_morph(value: &str) -> Result<()> {
    if value.is_empty() {
        return Ok(());
    }
    let mut previous_key = None;
    for field in value.split('|') {
        let Some((key, values)) = field.split_once('=') else {
            return Err(Error::Unsupported("malformed document morphology".into()));
        };
        if key.is_empty()
            || (key.eq_ignore_ascii_case("POS")
                && (key != "POS"
                    || (known_pos(&values.to_ascii_uppercase())
                        && values != values.to_ascii_uppercase())))
            || previous_key.is_some_and(|previous| previous >= key)
            || field.chars().any(char::is_whitespace)
            || values.contains('=')
        {
            return Err(Error::Unsupported(
                "matcher requires canonical morphology".into(),
            ));
        }
        previous_key = Some(key);
        let mut previous_value = None;
        for item in values.split(',') {
            if item.is_empty() || previous_value.is_some_and(|previous| previous > item) {
                return Err(Error::Unsupported(
                    "matcher requires canonical morphology values".into(),
                ));
            }
            previous_value = Some(item);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
