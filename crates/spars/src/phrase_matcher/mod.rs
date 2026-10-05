//! Native phrase matching on token text or token annotations; no model or shared vocabulary required.
mod terminal;
use crate::dependency_matcher::predicates::validate_canonical_morph;
use crate::{Doc, Error, Result, Token, TokenIndex};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;
use terminal::Terminal;

/// String rule identity. Reserved spaCy symbols are resolved internally for ordering.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PhraseRuleId(pub String);
impl From<String> for PhraseRuleId {
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl From<&str> for PhraseRuleId {
    fn from(value: &str) -> Self {
        Self(value.into())
    }
}
impl std::borrow::Borrow<str> for PhraseRuleId {
    fn borrow(&self) -> &str {
        &self.0
    }
}

/// The token value a [`PhraseMatcher`] compares, named as in spaCy.
///
/// ORTH and TEXT match exact text; LOWER uses pinned Python Unicode lowercase. NORM uses the
/// token's norm. LEMMA, POS, TAG, DEP and MORPH compare annotations: a missing value is the
/// empty string, as in spaCy, and the empty morphological analysis is `_`.
// TODO(phrase-attributes): add spaCy's lexical flag, LENGTH, SHAPE, entity, sentence-start and
// SPACY attributes. Tracked in docs/PHRASE_MATCHER.md#rust-boundaries-and-remaining-scope
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum PhraseAttribute {
    #[default]
    Orth,
    Text,
    Lower,
    Norm,
    Lemma,
    Pos,
    Tag,
    Dep,
    Morph,
}

// spaCy stores no value as key 0, whose string is empty, and the empty
// morphological analysis as its own symbol `_`.
const MISSING: &str = "";
const EMPTY_MORPH: &str = "_";

impl PhraseAttribute {
    /// The annotation name used in missing-annotation errors, for attributes that
    /// spaCy requires on pattern documents.
    fn required_annotation(self) -> Option<&'static str> {
        match self {
            Self::Lemma => Some("lemma"),
            Self::Pos => Some("POS"),
            Self::Tag => Some("tag"),
            Self::Dep => Some("dependency label"),
            Self::Morph => Some("morphology"),
            Self::Orth | Self::Text | Self::Lower | Self::Norm => None,
        }
    }
    /// The stored value for NORM and the annotation attributes, as spaCy's string for its key.
    fn annotation(self, token: &Token) -> &str {
        match self {
            Self::Norm => &token.norm,
            Self::Lemma => token.lemma.as_deref().unwrap_or(MISSING),
            Self::Pos => token.pos.as_deref().unwrap_or(MISSING),
            Self::Tag => token.tag.as_deref().unwrap_or(MISSING),
            Self::Dep => token.dep.as_deref().unwrap_or(MISSING),
            Self::Morph => match token.morphology.as_deref() {
                None => MISSING,
                Some("") => EMPTY_MORPH,
                Some(value) => value,
            },
            Self::Orth | Self::Text | Self::Lower => {
                unreachable!("text attributes read token text")
            }
        }
    }
    // Document morphology is compared as stored, so it must be in spaCy's canonical order.
    fn validate_document(self, doc: &Doc) -> Result<()> {
        if self == Self::Morph {
            for token in doc.tokens() {
                if let Some(value) = token.morphology.as_deref() {
                    validate_canonical_morph(value)?;
                }
            }
        }
        Ok(())
    }
}

/// Owned, unique pattern of token values retained for rule lookup.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct PhrasePattern {
    tokens: Vec<String>,
}
impl PhrasePattern {
    /// The compared value of each pattern token, as described on [`PhraseAttribute`].
    pub fn tokens(&self) -> &[String] {
        &self.tokens
    }
    fn from_doc(doc: &Doc, attribute: PhraseAttribute) -> Result<Self> {
        let tokens: Vec<String> = match attribute {
            PhraseAttribute::Orth | PhraseAttribute::Text | PhraseAttribute::Lower => {
                (0..doc.tokens().len())
                    .map(|i| {
                        doc.token_text(TokenIndex(i)).map(|text| match attribute {
                            PhraseAttribute::Lower => crate::unicode_lower::lower(text),
                            _ => text.to_owned(),
                        })
                    })
                    .collect::<Result<_>>()?
            }
            _ => {
                attribute.validate_document(doc)?;
                doc.tokens()
                    .iter()
                    .map(|token| attribute.annotation(token).to_owned())
                    .collect()
            }
        };
        // Like spaCy, a nonempty pattern needs at least one annotated token; empty
        // patterns are skipped before this check.
        if let Some(name) = attribute.required_annotation() {
            if !tokens.is_empty() && tokens.iter().all(|value| value == MISSING) {
                return Err(Error::MissingAnnotation(name));
            }
        }
        Ok(Self { tokens })
    }
}
/// A phrase occurrence; end is an exclusive document token index.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhraseMatch {
    pub rule: PhraseRuleId,
    pub start: TokenIndex,
    pub end: TokenIndex,
}
#[derive(Default)]
struct Node {
    children: HashMap<String, usize>,
    terminal: Option<Terminal>,
}
struct Rule {
    key: u64,
    patterns: Vec<PhrasePattern>,
    // An endpoint stays allocated while this rule owns its terminal.
    unique: HashSet<usize>,
}
/// Reusable prefix trie. Read-only searches own their output and share no mutable state.
pub struct PhraseMatcher {
    attribute: PhraseAttribute,
    rules: HashMap<PhraseRuleId, Rule>,
    identities: HashMap<u64, PhraseRuleId>,
    nodes: Vec<Node>,
    free: Vec<usize>,
}
impl Default for PhraseMatcher {
    fn default() -> Self {
        Self::with_attribute(PhraseAttribute::Orth)
    }
}
impl PhraseMatcher {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with_attribute(attribute: PhraseAttribute) -> Self {
        Self {
            attribute,
            rules: HashMap::new(),
            identities: HashMap::new(),
            nodes: vec![Node::default()],
            free: Vec::new(),
        }
    }
    pub fn attribute(&self) -> PhraseAttribute {
        self.attribute
    }
    pub fn len(&self) -> usize {
        self.rules.len()
    }
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }
    pub fn contains(&self, name: &str) -> bool {
        self.rules.contains_key(name)
    }
    /// Unique nonempty patterns in first-registration order (a Rust inspection API).
    pub fn get(&self, name: &str) -> Option<&[PhrasePattern]> {
        self.rules.get(name).map(|r| r.patterns.as_slice())
    }
    /// Copy the selected token values from validated documents. Empty patterns are ignored but
    /// register the rule. LEMMA, POS, TAG, DEP and MORPH patterns need at least one annotated token.
    /// All fallible validation completes before mutation; duplicate additions still update terminal order.
    pub fn add(&mut self, name: impl Into<PhraseRuleId>, documents: &[&Doc]) -> Result<()> {
        let name = name.into();
        let key = rule_key(&name.0);
        if self
            .identities
            .get(&key)
            .is_some_and(|other| other != &name)
        {
            return Err(Error::Pattern("phrase rule hash collision".into()));
        }
        let patterns = documents
            .iter()
            .map(|doc| PhrasePattern::from_doc(doc, self.attribute))
            .collect::<Result<Vec<_>>>()?;
        self.identities.insert(key, name.clone());
        self.rules.entry(name.clone()).or_insert_with(|| Rule {
            key,
            patterns: Vec::new(),
            unique: HashSet::new(),
        });
        for pattern in patterns {
            if pattern.tokens.is_empty() {
                continue;
            }
            let mut node = 0;
            for token in &pattern.tokens {
                node = if let Some(&child) = self.nodes[node].children.get(token) {
                    child
                } else {
                    let child = self.free.pop().unwrap_or_else(|| {
                        self.nodes.push(Node::default());
                        self.nodes.len() - 1
                    });
                    self.nodes[node].children.insert(token.clone(), child);
                    child
                };
            }
            self.nodes[node]
                .terminal
                .get_or_insert_with(Terminal::default)
                .insert(key, name.clone());
            let rule = self.rules.get_mut(&name).expect("registered rule");
            if rule.unique.insert(node) {
                rule.patterns.push(pattern);
            }
        }
        Ok(())
    }
    /// Remove a rule and reclaim its unused trie nodes. Unknown names return an error.
    pub fn remove(&mut self, name: &str) -> Result<()> {
        let rule = self
            .rules
            .remove(name)
            .ok_or_else(|| Error::Pattern(format!("unknown phrase rule {name:?}")))?;
        self.identities.remove(&rule.key);
        for pattern in &rule.patterns {
            let mut node = 0;
            let mut path = Vec::with_capacity(pattern.tokens.len());
            for token in &pattern.tokens {
                let child = self.nodes[node].children[token];
                path.push((node, token, child));
                node = child;
            }
            if let Some(terminal) = &mut self.nodes[node].terminal {
                terminal.remove(rule.key);
                if terminal.rules().next().is_none() {
                    self.nodes[node].terminal = None;
                }
            }
            for (parent, token, child) in path.into_iter().rev() {
                if self.nodes[child].terminal.is_some() || !self.nodes[child].children.is_empty() {
                    break;
                }
                self.nodes[parent].children.remove(token);
                self.nodes[child] = Node::default();
                self.free.push(child);
            }
        }
        Ok(())
    }
    /// Return all overlapping occurrences in pinned spaCy start/end/terminal-table order.
    ///
    /// Like spaCy, the input needs no annotations: a token without the selected
    /// annotation matches only a pattern token that also lacks it.
    pub fn find_matches(&self, doc: &Doc) -> Result<Vec<PhraseMatch>> {
        // Validate even without rules, so the outcome does not depend on matcher state.
        self.attribute.validate_document(doc)?;
        if self.rules.is_empty() {
            return Ok(Vec::new());
        }
        // Dispatch once so exact matching does not branch on attributes for
        // every trie edge. LOWER keys are computed once per input token.
        match self.attribute {
            PhraseAttribute::Orth | PhraseAttribute::Text => self
                .find_keys(doc.tokens().len(), |index| {
                    doc.token_text(TokenIndex(index))
                }),
            PhraseAttribute::Lower => {
                let lowered = (0..doc.tokens().len())
                    .map(|i| {
                        doc.token_text(TokenIndex(i))
                            .map(crate::unicode_lower::lower)
                    })
                    .collect::<Result<Vec<_>>>()?;
                self.find_keys(doc.tokens().len(), |index| Ok(lowered[index].as_str()))
            }
            attribute => {
                let tokens = doc.tokens();
                self.find_keys(tokens.len(), |index| {
                    Ok(attribute.annotation(&tokens[index]))
                })
            }
        }
    }

    fn find_keys<'a>(
        &self,
        count: usize,
        key: impl Fn(usize) -> Result<&'a str>,
    ) -> Result<Vec<PhraseMatch>> {
        let mut matches = Vec::new();
        for start in 0..count {
            let mut node = 0;
            for end in start..count {
                let Some(&child) = self.nodes[node].children.get(key(end)?) else {
                    break;
                };
                node = child;
                if let Some(terminal) = &self.nodes[node].terminal {
                    matches.extend(terminal.rules().map(|rule| PhraseMatch {
                        rule: rule.clone(),
                        start: TokenIndex(start),
                        end: TokenIndex(end + 1),
                    }));
                }
            }
        }
        Ok(matches)
    }
}
fn rule_key(name: &str) -> u64 {
    static SYMBOLS: OnceLock<HashMap<String, u64>> = OnceLock::new();
    *SYMBOLS
        .get_or_init(|| {
            serde_json::from_str(include_str!("symbols.json")).expect("pinned spaCy symbols")
        })
        .get(name)
        .unwrap_or(&crate::hash::hash(name))
}
#[cfg(test)]
mod tests;
