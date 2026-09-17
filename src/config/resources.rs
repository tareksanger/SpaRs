use serde::Deserialize;
use std::collections::HashMap;
#[derive(Deserialize)]
pub(crate) struct TokenizerConfig {
    pub prefix: String,
    pub suffix: String,
    pub infix: String,
    pub url: String,
    pub token_match: Option<String>,
    #[serde(default, deserialize_with = "present")]
    pub faster_heuristics: Option<bool>,
    pub regex_dialect: String,
    pub rules: HashMap<String, Vec<Exception>>,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Exception {
    #[serde(rename = "ORTH")]
    pub orth: String,
    #[serde(rename = "NORM")]
    pub norm: Option<String>,
}
#[derive(Deserialize)]
pub(crate) struct Lexical {
    pub email_regex: String,
    pub lower: HashMap<String, String>,
    pub ranges: Ranges,
    pub stops: Vec<String>,
    pub number_words: Vec<String>,
    pub tlds: Vec<String>,
    pub is_bracket: Vec<String>,
    pub is_quote: Vec<String>,
    pub is_left_punct: Vec<String>,
    pub is_right_punct: Vec<String>,
}
#[derive(Clone, Copy)]
pub(crate) enum CharFlag {
    Alpha,
    Digit,
    Lower,
    Upper,
    Title,
    Punct,
    Currency,
    CaseIgnorable,
}
#[derive(Deserialize)]
pub(crate) struct Ranges {
    pub alpha: Vec<[u32; 2]>,
    pub digit: Vec<[u32; 2]>,
    pub lower: Vec<[u32; 2]>,
    pub upper: Vec<[u32; 2]>,
    pub title: Vec<[u32; 2]>,
    pub punct: Vec<[u32; 2]>,
    pub currency: Vec<[u32; 2]>,
    pub case_ignorable: Vec<[u32; 2]>,
}
impl Ranges {
    pub fn get(&self, f: CharFlag) -> &[[u32; 2]] {
        match f {
            CharFlag::Alpha => &self.alpha,
            CharFlag::Digit => &self.digit,
            CharFlag::Lower => &self.lower,
            CharFlag::Upper => &self.upper,
            CharFlag::Title => &self.title,
            CharFlag::Punct => &self.punct,
            CharFlag::Currency => &self.currency,
            CharFlag::CaseIgnorable => &self.case_ignorable,
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AttributeRule {
    pub index: i64,
    pub patterns: Vec<Vec<TokenPattern>>,
    pub attrs: HashMap<OutputAttribute, String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "SCREAMING_SNAKE_CASE")]
pub(crate) struct TokenPattern {
    #[serde(default, deserialize_with = "present")]
    pub lower: Option<Constraint>,
    #[serde(default, deserialize_with = "present")]
    pub tag: Option<Constraint>,
    #[serde(default, deserialize_with = "present")]
    pub dep: Option<Constraint>,
    #[serde(default, deserialize_with = "present")]
    pub is_space: Option<bool>,
}
#[derive(Deserialize)]
#[serde(untagged)]
pub(crate) enum Constraint {
    Exact(String),
    Operators(Operators),
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "SCREAMING_SNAKE_CASE")]
pub(crate) struct Operators {
    #[serde(rename = "IN")]
    #[serde(default, deserialize_with = "present")]
    pub included: Option<Vec<String>>,
    #[serde(default, deserialize_with = "present")]
    pub not_in: Option<Vec<String>>,
    #[serde(default, deserialize_with = "present")]
    pub regex: Option<String>,
}
#[derive(Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub(crate) enum OutputAttribute {
    Pos,
    Tag,
    Morph,
    Lemma,
    Dep,
}
#[derive(Deserialize)]
pub(crate) struct Lemmas {
    #[serde(deserialize_with = "tables")]
    pub lemma_index: HashMap<String, Vec<String>>,
    pub lemma_exc: HashMap<String, HashMap<String, Vec<String>>>,
    #[serde(deserialize_with = "tables")]
    pub lemma_rules: HashMap<String, Vec<[String; 2]>>,
}
fn tables<'de, D, T>(d: D) -> Result<HashMap<String, Vec<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Table<T> {
        Items(Vec<T>),
        Empty(HashMap<String, serde::de::IgnoredAny>),
    }
    let raw = HashMap::<String, Table<T>>::deserialize(d)?;
    raw.into_iter()
        .map(|(k, v)| {
            Ok((
                k,
                match v {
                    Table::Items(v) => v,
                    Table::Empty(v) if v.is_empty() => Vec::new(),
                    Table::Empty(_) => {
                        return Err(serde::de::Error::custom("expected list or empty table"))
                    }
                },
            ))
        })
        .collect()
}

// A missing optional field is different from an explicitly null field.
fn present<'de, D, T>(d: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(d).map(Some)
}
