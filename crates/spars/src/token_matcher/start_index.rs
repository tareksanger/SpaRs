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
/// requires. Every list is ascending. Each pattern with nodes is in at most one group, and in
/// none when its first node requires a value from an empty set.
#[derive(Default)]
pub(super) struct StartIndex {
    text: HashMap<Box<str>, Vec<usize>>,
    lower: HashMap<Box<str>, Vec<usize>>,
    norm: HashMap<Box<str>, Vec<usize>>,
    // Patterns whose first node can match without an exact text value, or can be skipped.
    unindexed: Vec<usize>,
    // One more than the last position passed to `push`, reduced by `remove`.
    next: usize,
}

impl StartIndex {
    /// Record the pattern at `position`, which must exceed every position recorded so far.
    pub(super) fn push(&mut self, position: usize, pattern: &CompiledPattern) {
        debug_assert!(
            position >= self.next,
            "positions must be recorded in ascending order"
        );
        self.next = position + 1;
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
            // Look up before inserting, so a repeated value does not allocate a new key.
            match map.get_mut(value) {
                Some(positions) => positions.push(position),
                None => {
                    map.insert(value.into(), vec![position]);
                }
            }
        }
    }

    /// Drop the patterns at the ascending positions `removed` and shift later positions down,
    /// as removing them from the matcher's compiled list does.
    pub(super) fn remove(&mut self, removed: &[usize]) {
        if removed.is_empty() {
            return;
        }
        // Both lists ascend, so one merge pass renumbers a list; entries before the first
        // removed position are unchanged and skipped.
        let renumber = |positions: &mut Vec<usize>| {
            let unchanged = positions.partition_point(|&position| position < removed[0]);
            let mut kept = unchanged;
            let mut before = 0;
            for read in unchanged..positions.len() {
                let position = positions[read];
                while removed.get(before).is_some_and(|&gone| gone < position) {
                    before += 1;
                }
                if removed.get(before) != Some(&position) {
                    positions[kept] = position - before;
                    kept += 1;
                }
            }
            positions.truncate(kept);
        };
        for map in [&mut self.text, &mut self.lower, &mut self.norm] {
            map.retain(|_, positions| {
                renumber(positions);
                !positions.is_empty()
            });
        }
        renumber(&mut self.unindexed);
        self.next = self.next.saturating_sub(removed.len());
    }

    /// Every position reached through a value lookup rather than at every token.
    #[cfg(test)]
    pub(super) fn indexed_positions(&self) -> std::collections::HashSet<usize> {
        [&self.text, &self.lower, &self.norm]
            .into_iter()
            .flat_map(|map| map.values().flatten().copied())
            .collect()
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
