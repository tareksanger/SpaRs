//! Native exact-token-text phrase matching; no model or shared vocabulary required.
mod lower;
mod terminal;
use crate::{Doc, Error, Result, TokenIndex};
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

/// ORTH and TEXT match exact text; LOWER uses pinned Python Unicode lowercase.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum PhraseAttribute {
    #[default]
    Orth,
    Text,
    Lower,
}

/// Owned, unique token-text pattern retained for rule lookup.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct PhrasePattern {
    tokens: Vec<String>,
}
impl PhrasePattern {
    pub fn tokens(&self) -> &[String] {
        &self.tokens
    }
    fn from_doc(doc: &Doc, attribute: PhraseAttribute) -> Result<Self> {
        Ok(Self {
            tokens: (0..doc.tokens().len())
                .map(|i| {
                    doc.token_text(TokenIndex(i)).map(|text| match attribute {
                        PhraseAttribute::Lower => lower::lower(text),
                        _ => text.to_owned(),
                    })
                })
                .collect::<Result<_>>()?,
        })
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
    /// Copy token texts from validated documents. Empty patterns are ignored but register the rule.
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
    pub fn find_matches(&self, doc: &Doc) -> Result<Vec<PhraseMatch>> {
        let mut matches = Vec::new();
        if self.rules.is_empty() {
            return Ok(matches);
        }
        // Normalize each input token once; shared trie prefixes reuse the keys.
        let lowered = if self.attribute == PhraseAttribute::Lower {
            Some(
                (0..doc.tokens().len())
                    .map(|i| doc.token_text(TokenIndex(i)).map(lower::lower))
                    .collect::<Result<Vec<_>>>()?,
            )
        } else {
            None
        };
        for start in 0..doc.tokens().len() {
            let mut node = 0;
            for end in start..doc.tokens().len() {
                let Some(&child) = self.nodes[node].children.get(match &lowered {
                    Some(tokens) => tokens[end].as_str(),
                    None => doc.token_text(TokenIndex(end))?,
                }) else {
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
