//! Reusable patterns over contiguous tokens, with explicit repetition.
mod engine;
use crate::dependency_matcher::predicates::CompiledConstraint;
use crate::{Doc, Error, Result, TokenConstraint, TokenIndex};
use serde::{Deserialize, Serialize};

/// An ordered sequence of token conditions.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TokenPattern {
    pub tokens: Vec<TokenPatternItem>,
}
/// Conjunctive conditions and the number of tokens they may consume.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TokenPatternItem {
    pub constraints: Vec<TokenConstraint>,
    pub repetition: Repetition,
}
/// Negation consumes one token whose conditions do not all match.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Repetition {
    #[default]
    Once,
    Optional,
    ZeroOrMore,
    OneOrMore,
    Negated,
    /// Inclusive limits; `None` permits an unbounded number of tokens.
    Range {
        min: usize,
        max: Option<usize>,
    },
}
/// A nonempty match with an exclusive end token index.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TokenMatch {
    pub rule: String,
    pub start: TokenIndex,
    pub end: TokenIndex,
}
#[derive(Clone, Copy)]
enum Quantifier {
    One,
    Optional,
    Star,
    Negated,
}
struct Node {
    item: usize,
    quantifier: Quantifier,
}
struct CompiledPattern {
    name: String,
    constraints: Vec<Vec<CompiledConstraint>>,
    nodes: Vec<Node>,
}
struct Rule {
    name: String,
    patterns: Vec<TokenPattern>,
}
/// Compile rules once and reuse them across independent document calls.
#[derive(Default)]
pub struct TokenMatcher {
    rules: Vec<Rule>,
    compiled: Vec<CompiledPattern>,
}
impl TokenMatcher {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn len(&self) -> usize {
        self.rules.len()
    }
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }
    pub fn contains(&self, name: &str) -> bool {
        self.rules.iter().any(|rule| rule.name == name)
    }
    pub fn get(&self, name: &str) -> Option<&[TokenPattern]> {
        self.rules
            .iter()
            .find(|rule| rule.name == name)
            .map(|rule| rule.patterns.as_slice())
    }
    /// Validate every pattern before modifying the matcher. Repeated names append patterns.
    pub fn add(&mut self, name: impl Into<String>, patterns: Vec<TokenPattern>) -> Result<()> {
        let name = name.into();
        let compiled = patterns
            .iter()
            .map(|pattern| compile(&name, pattern))
            .collect::<Result<Vec<_>>>()?;
        if let Some(rule) = self.rules.iter_mut().find(|rule| rule.name == name) {
            rule.patterns.extend(patterns);
        } else {
            self.rules.push(Rule { name, patterns });
        }
        // This order includes later additions to an already registered rule.
        self.compiled.extend(compiled);
        Ok(())
    }
    pub fn remove(&mut self, name: &str) -> Result<()> {
        let index = self
            .rules
            .iter()
            .position(|rule| rule.name == name)
            .ok_or_else(|| Error::Pattern(format!("unknown token rule {name:?}")))?;
        self.rules.remove(index);
        self.compiled.retain(|pattern| pattern.name != name);
        Ok(())
    }
    /// Return overlapping matches in the reference state-machine emission order.
    pub fn find_matches(&self, doc: &Doc) -> Result<Vec<TokenMatch>> {
        engine::find(&self.compiled, doc)
    }
}
fn compile(name: &str, pattern: &TokenPattern) -> Result<CompiledPattern> {
    const MAX_NODES: usize = 4096;
    if pattern.tokens.is_empty() {
        return Err(Error::Pattern("token pattern must contain an item".into()));
    }
    let mut nodes = Vec::new();
    let mut constraints = Vec::new();
    for (item, spec) in pattern.tokens.iter().enumerate() {
        let (required, optional, star, negated) = match spec.repetition {
            Repetition::Once => (1, 0, false, false),
            Repetition::Negated => (1, 0, false, true),
            Repetition::Optional => (0, 1, false, false),
            Repetition::ZeroOrMore => (0, 0, true, false),
            Repetition::OneOrMore => (1, 0, true, false),
            Repetition::Range { min, max } => match max {
                Some(max) if max >= min => (min, max - min, false, false),
                None => (min, 0, true, false),
                _ => {
                    return Err(Error::Pattern(
                        "repetition maximum must be at least its minimum".into(),
                    ))
                }
            },
        };
        let added = required
            .checked_add(optional)
            .and_then(|n| n.checked_add(usize::from(star)))
            .and_then(|n| n.checked_add(nodes.len()));
        if added.is_none_or(|n| n > MAX_NODES) {
            return Err(Error::Pattern(format!(
                "token pattern exceeds {MAX_NODES} expanded nodes"
            )));
        }
        let compiled = spec
            .constraints
            .iter()
            .map(TokenConstraint::compile)
            .collect::<Result<Vec<_>>>()?;
        constraints.push(if required == 0 && optional == 0 && !star {
            Vec::new()
        } else {
            compiled
        });
        nodes.extend((0..required).map(|_| Node {
            item,
            quantifier: if negated {
                Quantifier::Negated
            } else {
                Quantifier::One
            },
        }));
        nodes.extend((0..optional).map(|_| Node {
            item,
            quantifier: Quantifier::Optional,
        }));
        if star {
            nodes.push(Node {
                item,
                quantifier: Quantifier::Star,
            });
        }
    }
    Ok(CompiledPattern {
        name: name.into(),
        constraints,
        nodes,
    })
}
#[cfg(test)]
mod tests;
