//! Which compiled patterns can begin a match at a token.
//!
//! spaCy's `Matcher` starts a state for every pattern at every token. A state whose first node
//! consumes exactly one token fails immediately, without emitting or branching, when that token
//! fails the node's conditions. So a pattern whose first node requires an exact `Text`, `Lower`
//! or `Norm` value only needs a state at tokens with one of those values. Starting only those
//! states, in the same pattern order, leaves the emitted matches and their order unchanged.
use super::{CompiledPattern, Quantifier};
use crate::dependency_matcher::predicates::{TextAttribute, TokenValues};
use crate::TokenView;
use std::collections::HashMap;

/// Pattern positions in the matcher's compiled order, grouped by the value their first node
/// requires. Every list is ascending, and each pattern with nodes is in exactly one group.
#[derive(Default)]
pub(super) struct StartIndex {
    text: HashMap<Box<str>, Vec<usize>>,
    lower: HashMap<Box<str>, Vec<usize>>,
    norm: HashMap<Box<str>, Vec<usize>>,
    // Patterns whose first node can match without an exact text value, or can be skipped.
    unindexed: Vec<usize>,
}

impl StartIndex {
    /// Record the pattern at `position`, which must exceed every position recorded so far.
    pub(super) fn push(&mut self, position: usize, pattern: &CompiledPattern) {
        let Some(first) = pattern.nodes.first() else {
            // A pattern without nodes never starts a state.
            return;
        };
        // Optional, repeated-from-zero and negated nodes do not require their conditions.
        let key = matches!(first.quantifier, Quantifier::One)
            .then(|| {
                pattern.items[first.item]
                    .constraints
                    .iter()
                    .find_map(|constraint| constraint.exact_values())
            })
            .flatten();
        let Some((attribute, values)) = key else {
            self.unindexed.push(position);
            return;
        };
        let map = match attribute {
            TextAttribute::Text => &mut self.text,
            TextAttribute::Lower => &mut self.lower,
            TextAttribute::Norm => &mut self.norm,
        };
        // An empty set can never match, so the pattern needs no start state at all.
        for value in values {
            map.entry(value.into()).or_default().push(position);
        }
    }

    /// Keep the patterns whose old position `p` has `kept[p]`, renumbered as the matcher's
    /// compiled list is after `Vec::retain` with the same decisions.
    pub(super) fn retain(&mut self, kept: &[bool]) {
        let mut next = 0;
        let renumbered: Vec<Option<usize>> = kept
            .iter()
            .map(|&keep| {
                keep.then(|| {
                    next += 1;
                    next - 1
                })
            })
            .collect();
        let renumber = |positions: &mut Vec<usize>| {
            positions.retain_mut(|position| match renumbered[*position] {
                Some(new) => {
                    *position = new;
                    true
                }
                None => false,
            });
        };
        for map in [&mut self.text, &mut self.lower, &mut self.norm] {
            map.retain(|_, positions| {
                renumber(positions);
                !positions.is_empty()
            });
        }
        renumber(&mut self.unindexed);
    }

    /// Replace `out` with the patterns that may start a match at `token`, in ascending order.
    pub(super) fn candidates<'a>(
        &self,
        token: TokenView<'a>,
        values: &'a TokenValues<'a>,
        out: &mut Vec<usize>,
    ) {
        out.clear();
        let mut sources = [
            lookup(&self.text, TextAttribute::Text, token, values),
            lookup(&self.lower, TextAttribute::Lower, token, values),
            lookup(&self.norm, TextAttribute::Norm, token, values),
            &self.unindexed[..],
        ];
        let mut nonempty = sources.iter().filter(|positions| !positions.is_empty());
        if let (Some(only), None) = (nonempty.next(), nonempty.next()) {
            out.extend_from_slice(only);
            return;
        }
        // The groups are disjoint and ascending, so merging them yields compiled order.
        loop {
            let mut smallest: Option<(usize, usize)> = None;
            for (source, positions) in sources.iter().enumerate() {
                if let Some(&position) = positions.first() {
                    if smallest.is_none_or(|(_, best)| position < best) {
                        smallest = Some((source, position));
                    }
                }
            }
            let Some((source, position)) = smallest else {
                break;
            };
            out.push(position);
            sources[source] = &sources[source][1..];
        }
    }
}

// The patterns indexed under the token's value of `attribute`.
fn lookup<'m>(
    map: &'m HashMap<Box<str>, Vec<usize>>,
    attribute: TextAttribute,
    token: TokenView<'_>,
    values: &TokenValues<'_>,
) -> &'m [usize] {
    if map.is_empty() {
        return &[];
    }
    map.get(attribute.value(token, values).as_ref())
        .map_or(&[], Vec::as_slice)
}
