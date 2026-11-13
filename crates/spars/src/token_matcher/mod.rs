//! Reusable patterns over contiguous tokens, with explicit repetition.
mod engine;
mod greedy;
mod start_index;
use crate::dependency_matcher::predicates::{
    compile_conditions, CompiledConditions, LengthChecks, LexicalTest,
};
use crate::{Doc, Error, Lexicon, Result, TokenConstraint, TokenIndex};
use serde::{Deserialize, Serialize};
use start_index::StartIndex;
use std::collections::{HashMap, HashSet};

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
/// How a rule chooses among its own overlapping matches, set through
/// [`TokenRuleOptions::greedy`]; the values of spaCy's `greedy` argument of `Matcher.add`. The
/// rule's matches are taken in the variant's order of preference, and a match is kept when none
/// of its tokens belongs to a match kept before it, so no token belongs to two matches of the
/// rule. Matches of different rules never compete.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Greedy {
    /// spaCy's `"FIRST"`: prefer the match that starts earliest, then the longer one.
    First,
    /// spaCy's `"LONGEST"`: prefer the longest match, then the one that starts earliest.
    Longest,
}
/// Settings for one rule, passed to [`TokenMatcher::add_with`]. Start from
/// [`TokenRuleOptions::default`] and assign fields; code outside this crate cannot use a struct
/// literal.
///
/// ```
/// use spars::{Greedy, TokenMatcher, TokenRuleOptions};
///
/// let mut options = TokenRuleOptions::default();
/// options.greedy = Some(Greedy::Longest);
/// let mut matcher = TokenMatcher::new();
/// matcher.add_with("run", Vec::new(), options)?;
/// assert!(matcher.contains("run"));
/// # Ok::<(), spars::Error>(())
/// ```
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub struct TokenRuleOptions {
    /// `None` (the default) returns every match of the rule, overlapping ones included. `Some`
    /// keeps only matches that share no token, chosen as [`Greedy`] describes. Corresponds to
    /// spaCy's `greedy` argument of `Matcher.add`.
    pub greedy: Option<Greedy>,
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
    items: Vec<CompiledConditions>,
    lengths: Vec<LengthChecks>,
    nodes: Vec<Node>,
}
struct Rule {
    patterns: Vec<TokenPattern>,
    // Those of the latest registration of the rule; they apply to all of its patterns.
    options: TokenRuleOptions,
}
/// Compile rules once and reuse them across independent document calls.
#[derive(Default)]
pub struct TokenMatcher {
    rules: Vec<Rule>,
    // The position of each rule in `rules`, so lookups do not scan every rule.
    index: HashMap<String, usize>,
    compiled: Vec<CompiledPattern>,
    // Kept consistent with `compiled` by `add` and `remove`.
    starts: StartIndex,
    lexicon: Option<Lexicon>,
}
impl TokenMatcher {
    pub fn new() -> Self {
        Self::default()
    }
    /// A matcher that can also evaluate lexical flags, such as `LikeNum`, with a
    /// model's language rules. Obtain the lexicon with [`crate::Model::lexicon`].
    pub fn with_lexicon(lexicon: Lexicon) -> Self {
        Self {
            lexicon: Some(lexicon),
            ..Self::default()
        }
    }
    pub fn len(&self) -> usize {
        self.rules.len()
    }
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }
    pub fn contains(&self, name: &str) -> bool {
        self.index.contains_key(name)
    }
    pub fn get(&self, name: &str) -> Option<&[TokenPattern]> {
        self.index
            .get(name)
            .map(|&i| self.rules[i].patterns.as_slice())
    }
    /// Register `patterns` under the rule `name`; a name that is already registered gets the
    /// patterns appended. Every pattern is validated first, and a call that returns an error
    /// changes nothing.
    ///
    /// This is [`TokenMatcher::add_with`] with the default options, so it also resets the options
    /// of an existing rule: a rule registered with [`TokenRuleOptions::greedy`] goes back to
    /// returning every match. See `add_with` for the errors.
    pub fn add(&mut self, name: impl Into<String>, patterns: Vec<TokenPattern>) -> Result<()> {
        self.add_with(name, patterns, TokenRuleOptions::default())
    }
    /// [`TokenMatcher::add`] with options for the rule. As in spaCy, the options of the latest
    /// registration of `name` replace the earlier ones and apply to every pattern of the rule,
    /// those added before included. An empty `patterns` adds no pattern but still sets the
    /// options, and registers `name` if it is new.
    ///
    /// Returns [`Error::Pattern`] when a pattern has no items, a `Range` has `max` below `min`, a
    /// pattern expands beyond 4096 nodes, a condition is invalid (for example a numeric predicate
    /// on an attribute other than `Length`), or a lexical flag condition is used on a matcher
    /// created without [`TokenMatcher::with_lexicon`]. A registration that returns an error
    /// leaves the rule's patterns and options as they were.
    pub fn add_with(
        &mut self,
        name: impl Into<String>,
        patterns: Vec<TokenPattern>,
        options: TokenRuleOptions,
    ) -> Result<()> {
        let name = name.into();
        self.check_lexicon(&patterns)?;
        let compiled = patterns
            .iter()
            .map(|pattern| compile(&name, pattern))
            .collect::<Result<Vec<_>>>()?;
        if let Some(&i) = self.index.get(&name) {
            let rule = &mut self.rules[i];
            rule.patterns.extend(patterns);
            rule.options = options;
        } else {
            self.index.insert(name, self.rules.len());
            self.rules.push(Rule { patterns, options });
        }
        // This order includes later additions to an already registered rule.
        for pattern in compiled {
            self.starts.push(self.compiled.len(), &pattern);
            self.compiled.push(pattern);
        }
        Ok(())
    }
    /// Return the error [`TokenMatcher::add`] would return for these patterns, without adding them.
    pub(crate) fn check(&self, name: &str, patterns: &[TokenPattern]) -> Result<()> {
        self.check_lexicon(patterns)?;
        patterns
            .iter()
            .try_for_each(|pattern| compile(name, pattern).map(drop))
    }
    fn check_lexicon(&self, patterns: &[TokenPattern]) -> Result<()> {
        if self.lexicon.is_none()
            && patterns
                .iter()
                .flat_map(|pattern| &pattern.tokens)
                .flat_map(|item| &item.constraints)
                .any(|constraint| constraint.attribute.needs_lexicon())
        {
            return Err(Error::Pattern(
                "lexical flag conditions require a matcher created with a lexicon".into(),
            ));
        }
        Ok(())
    }
    pub fn remove(&mut self, name: &str) -> Result<()> {
        let index = self
            .index
            .remove(name)
            .ok_or_else(|| Error::Pattern(format!("unknown token rule {name:?}")))?;
        // Each registered pattern has exactly one compiled pattern.
        let count = self.rules.remove(index).patterns.len();
        // Visiting the values directly avoids hashing every later rule name.
        for position in self.index.values_mut() {
            if *position > index {
                *position -= 1;
            }
        }
        let removed: Vec<usize> = (0..self.compiled.len())
            .filter(|&position| self.compiled[position].name == name)
            .take(count)
            .collect();
        self.starts.remove(&removed);
        if let [only] = removed[..] {
            self.compiled.remove(only);
        } else {
            let mut position = 0;
            self.compiled.retain(|_| {
                position += 1;
                removed.binary_search(&(position - 1)).is_err()
            });
        }
        Ok(())
    }
    /// Remove every registered rule in `names`, ignoring unknown names, in one pass over the rules
    /// and compiled patterns; removing rules one at a time costs a pass each.
    pub(crate) fn remove_all(&mut self, names: &HashSet<&str>) {
        let mut kept = vec![true; self.rules.len()];
        self.index.retain(|name, &mut position| {
            let keep = !names.contains(name.as_str());
            kept[position] = keep;
            keep
        });
        // Each kept rule moves down by the number of removed rules before it.
        let mut shift = Vec::with_capacity(kept.len());
        let mut dropped = 0;
        for &keep in &kept {
            shift.push(dropped);
            dropped += usize::from(!keep);
        }
        for position in self.index.values_mut() {
            *position -= shift[*position];
        }
        let mut position = 0;
        self.rules.retain(|_| {
            position += 1;
            kept[position - 1]
        });
        let removed: Vec<usize> = (0..self.compiled.len())
            .filter(|&position| names.contains(self.compiled[position].name.as_str()))
            .collect();
        self.starts.remove(&removed);
        self.compiled
            .retain(|pattern| !names.contains(pattern.name.as_str()));
    }
    /// Return the matches in spaCy `Matcher` order rather than sorted by start: every match,
    /// overlapping ones included, of each rule without [`TokenRuleOptions::greedy`]. A matcher
    /// without rules returns no matches and no error.
    ///
    /// A rule registered with [`TokenRuleOptions::greedy`] returns only matches that share no
    /// token with each other, chosen in the order of preference its [`Greedy`] value gives. As in
    /// spaCy, the result then lists the matches of rules without the option first, followed by
    /// the kept matches of each rule with it, one rule after another in the order in which the
    /// search first finds a match of each rule, and most preferred first within a rule.
    ///
    /// Returns [`Error::MissingAnnotation`] when an `Equals` condition compares lemma, POS, tag,
    /// dependency label or morphology and no token of a nonempty `doc` has that annotation,
    /// like spaCy's error E155; with several such attributes it names the first missing one in
    /// spaCy's order: tag, POS, morphology, lemma, dependency label. An empty lemma, POS, tag or
    /// label counts as missing; empty morphology counts as present. Other predicates never
    /// require the annotation, and a token without it compares as the empty string (see the
    /// Token Matcher guide for missing morphology). Returns [`Error::Unsupported`] when any
    /// condition compares morphology and a token's morphology is not in spaCy's canonical
    /// `Field=Value|...` order.
    pub fn find_matches(&self, doc: &Doc) -> Result<Vec<TokenMatch>> {
        let matches = engine::find(
            &self.compiled,
            engine::Starts::Indexed(&self.starts),
            doc,
            self.lexicon.as_ref(),
        )?;
        Ok(self.keep_greedy(matches))
    }
    /// Apply each rule's `greedy` option, in spaCy's result order: the matches of rules without
    /// the option come first, in search order; the kept matches of each rule with the option
    /// follow, rule by rule in the order of the rules' first matches.
    fn keep_greedy(&self, matches: Vec<TokenMatch>) -> Vec<TokenMatch> {
        if self.rules.iter().all(|rule| rule.options.greedy.is_none()) {
            return matches;
        }
        let mut kept = Vec::with_capacity(matches.len());
        let mut filtered: Vec<(Greedy, Vec<TokenMatch>)> = Vec::new();
        // For each rule, where its matches are collected in `filtered`.
        let mut position_of_rule: Vec<Option<usize>> = vec![None; self.rules.len()];
        for found in matches {
            // Every match names a registered rule.
            let rule = self.index[&found.rule];
            match self.rules[rule].options.greedy {
                None => kept.push(found),
                Some(greedy) => {
                    let position = *position_of_rule[rule].get_or_insert_with(|| {
                        filtered.push((greedy, Vec::new()));
                        filtered.len() - 1
                    });
                    filtered[position].1.push(found);
                }
            }
        }
        for (greedy, candidates) in filtered {
            kept.extend(greedy::keep_non_overlapping(greedy, candidates));
        }
        kept
    }
    /// The same search, starting every pattern at every token as spaCy does.
    #[cfg(test)]
    fn find_matches_unindexed(&self, doc: &Doc) -> Result<Vec<TokenMatch>> {
        let matches = engine::find(
            &self.compiled,
            engine::Starts::All,
            doc,
            self.lexicon.as_ref(),
        )?;
        Ok(self.keep_greedy(matches))
    }
}
fn compile(name: &str, pattern: &TokenPattern) -> Result<CompiledPattern> {
    const MAX_NODES: usize = 4096;
    if pattern.tokens.is_empty() {
        return Err(Error::Pattern("token pattern must contain an item".into()));
    }
    let mut nodes = Vec::new();
    let mut items = Vec::new();
    let mut lengths = Vec::new();
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
        let compiled = compile_conditions(&spec.constraints, &mut lengths)?;
        items.push(if required == 0 && optional == 0 && !star {
            CompiledConditions {
                constraints: Vec::new(),
                lexical: LexicalTest::default(),
            }
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
        items,
        lengths,
        nodes,
    })
}
#[cfg(test)]
mod tests;
