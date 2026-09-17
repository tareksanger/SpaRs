//! Reusable patterns over dependency relationships, evaluated without model access.
mod predicates;
mod relations;
use crate::{Doc, Error, Result, TokenIndex};
use predicates::CompiledConstraint;
pub use predicates::{Predicate, TokenAttribute, TokenConstraint};
use relations::Graph;
pub use relations::Relation;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// An ordered dependency pattern. Every link refers to an earlier node ID.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DependencyPattern {
    pub nodes: Vec<DependencyNode>,
}
/// A named node with conjunctive token constraints. Empty constraints match any token.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DependencyNode {
    pub id: String,
    pub constraints: Vec<TokenConstraint>,
    pub link: Option<DependencyLink>,
}
/// The relationship from an earlier node to this node.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DependencyLink {
    pub left: String,
    pub relation: Relation,
}
/// A match, with token indices in the pattern's node order.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DependencyMatch {
    pub rule: String,
    pub tokens: Vec<TokenIndex>,
}
struct CompiledNode {
    constraints: Vec<CompiledConstraint>,
    link: Option<(usize, Relation)>,
}
struct CompiledPattern {
    nodes: Vec<CompiledNode>,
}
struct Rule {
    name: String,
    patterns: Vec<DependencyPattern>,
    compiled: Vec<CompiledPattern>,
}
/// Immutable matching after rule registration. Calls use independent scratch state.
#[derive(Default)]
pub struct DependencyMatcher {
    rules: Vec<Rule>,
}
impl DependencyMatcher {
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
        self.rules.iter().any(|r| r.name == name)
    }
    pub fn get(&self, name: &str) -> Option<&[DependencyPattern]> {
        self.rules
            .iter()
            .find(|r| r.name == name)
            .map(|r| r.patterns.as_slice())
    }
    /// Validate all patterns before registering any of them. Existing rules append patterns.
    pub fn add(&mut self, name: impl Into<String>, patterns: Vec<DependencyPattern>) -> Result<()> {
        let name = name.into();
        let compiled = patterns.iter().map(compile).collect::<Result<Vec<_>>>()?;
        if let Some(rule) = self.rules.iter_mut().find(|r| r.name == name) {
            rule.patterns.extend(patterns);
            rule.compiled.extend(compiled);
        } else {
            self.rules.push(Rule {
                name,
                patterns,
                compiled,
            });
        }
        Ok(())
    }
    pub fn remove(&mut self, name: &str) -> Result<()> {
        let index = self
            .rules
            .iter()
            .position(|r| r.name == name)
            .ok_or_else(|| Error::Pattern(format!("unknown dependency rule {name:?}")))?;
        self.rules.remove(index);
        Ok(())
    }
    /// Find matches in official rule, pattern, root-bucket, and candidate order.
    /// Nonempty matchers require valid dependency heads. Position relations use
    /// dependency-root bounds, independently of stored sentence annotations.
    pub fn find_matches(&self, doc: &Doc) -> Result<Vec<DependencyMatch>> {
        if self.is_empty() {
            return Ok(Vec::new());
        }
        let mut graph = Graph::new(doc)?;
        let patterns: Vec<_> = self.rules.iter().flat_map(|r| &r.compiled).collect();
        let mut checked_attributes = HashSet::new();
        for pattern in &patterns {
            for node in &pattern.nodes {
                for constraint in &node.constraints {
                    if checked_attributes.insert(constraint.attribute()) {
                        constraint.validate_document(doc)?;
                    }
                }
            }
        }
        let mut candidates: Vec<HashMap<usize, Vec<Vec<usize>>>> =
            patterns.iter().map(|_| HashMap::new()).collect();
        let mut roots = Vec::new();
        let mut seen = vec![false; doc.tokens().len()];
        // Official Matcher emits token matches in document order. A root bucket
        // is inserted on its first match against any node in any rule.
        for index in 0..doc.tokens().len() {
            let token = doc.token(TokenIndex(index))?;
            let root = graph.roots[index];
            for (pattern, positions) in patterns.iter().zip(&mut candidates) {
                for (node_index, node) in pattern.nodes.iter().enumerate() {
                    let mut matches = true;
                    for constraint in &node.constraints {
                        if !constraint.matches(token)? {
                            matches = false;
                            break;
                        }
                    }
                    if matches {
                        if !seen[root] {
                            roots.push(root);
                            seen[root] = true;
                        }
                        positions
                            .entry(root)
                            .or_insert_with(|| vec![Vec::new(); pattern.nodes.len()])[node_index]
                            .push(index);
                    }
                }
            }
        }
        let mut output = Vec::new();
        let mut pattern_index = 0;
        for rule in &self.rules {
            for pattern in &rule.compiled {
                for root in &roots {
                    if let Some(positions) = candidates[pattern_index].get(root) {
                        search(pattern, positions, &mut graph, &rule.name, &mut output)?;
                    }
                }
                pattern_index += 1;
            }
        }
        Ok(output)
    }
}
fn compile(pattern: &DependencyPattern) -> Result<CompiledPattern> {
    if pattern.nodes.is_empty() {
        return Err(Error::Pattern(
            "dependency pattern must contain a node".into(),
        ));
    }
    let mut ids = HashMap::new();
    let mut nodes = Vec::with_capacity(pattern.nodes.len());
    for (index, node) in pattern.nodes.iter().enumerate() {
        if ids.contains_key(&node.id) {
            return Err(Error::Pattern(format!("duplicate node ID {:?}", node.id)));
        }
        let link = match (index, &node.link) {
            (0, None) => None,
            (0, Some(_)) => return Err(Error::Pattern("first node cannot have a link".into())),
            (_, None) => {
                return Err(Error::Pattern(format!(
                    "node {:?} requires a link",
                    node.id
                )))
            }
            (_, Some(link)) => Some((
                *ids.get(&link.left).ok_or_else(|| {
                    Error::Pattern(format!("LEFT_ID {:?} must name an earlier node", link.left))
                })?,
                link.relation,
            )),
        };
        let constraints = node
            .constraints
            .iter()
            .map(TokenConstraint::compile)
            .collect::<Result<Vec<_>>>()?;
        ids.insert(&node.id, index);
        nodes.push(CompiledNode { constraints, link });
    }
    Ok(CompiledPattern { nodes })
}
fn search(
    pattern: &CompiledPattern,
    positions: &[Vec<usize>],
    graph: &mut Graph<'_>,
    rule: &str,
    output: &mut Vec<DependencyMatch>,
) -> Result<()> {
    if positions.iter().any(Vec::is_empty) {
        return Ok(());
    }
    let n = pattern.nodes.len();
    let mut choices = vec![Vec::new(); n];
    choices[0].clone_from(&positions[0]);
    let mut cursors = vec![0; n];
    let mut selected = vec![TokenIndex(0); n];
    let mut depth = 0;
    loop {
        if cursors[depth] == choices[depth].len() {
            if depth == 0 {
                break;
            }
            depth -= 1;
            continue;
        }
        selected[depth] = TokenIndex(choices[depth][cursors[depth]]);
        cursors[depth] += 1;
        if depth + 1 == n {
            output.push(DependencyMatch {
                rule: rule.to_owned(),
                tokens: selected.clone(),
            });
            continue;
        }
        depth += 1;
        cursors[depth] = 0;
        choices[depth].clear();
        let (left, relation) = pattern.nodes[depth].link.expect("validated link");
        graph.select(
            selected[left].0,
            relation,
            &positions[depth],
            &mut choices[depth],
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
