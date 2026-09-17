use crate::{Doc, Error, Result, TokenView};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

/// Token annotations supported by dependency patterns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenAttribute {
    Text,
    Norm,
    Lemma,
    Pos,
    Tag,
    Dep,
    Morphology,
}

/// A comparison applied to one token attribute. Set predicates apply to morphology.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Predicate {
    Equals { value: String },
    In { values: Vec<String> },
    NotIn { values: Vec<String> },
    MorphSuperset { values: Vec<String> },
    MorphIntersects { values: Vec<String> },
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
        let predicate = match &self.predicate {
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
        for i in 0..doc.tokens().len() {
            let value = self.value(doc.token(crate::TokenIndex(i))?)?;
            if self.attribute == TokenAttribute::Morphology {
                validate_canonical_morph(value)?;
            }
        }
        Ok(())
    }
    fn value<'a>(&self, token: TokenView<'a>) -> Result<&'a str> {
        let t = token.annotations();
        let (value, name) = match self.attribute {
            TokenAttribute::Text => return Ok(token.text()),
            TokenAttribute::Norm => return Ok(&t.norm),
            TokenAttribute::Lemma => (t.lemma.as_deref(), "lemma"),
            TokenAttribute::Pos => (t.pos.as_deref(), "POS"),
            TokenAttribute::Tag => (t.tag.as_deref(), "tag"),
            TokenAttribute::Dep => (t.dep.as_deref(), "dependency label"),
            TokenAttribute::Morphology => (t.morphology.as_deref(), "morphology"),
        };
        value.ok_or(Error::MissingAnnotation(name))
    }
    pub(crate) fn matches(&self, token: TokenView<'_>) -> Result<bool> {
        let value = self.value(token)?;
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
