//! spaCy's `greedy` filter over the matches of one rule, as `Matcher.__call__` applies it.
use super::{Greedy, TokenMatch};
use std::cmp::Reverse;

/// The matches of one rule that `greedy` keeps, most preferred first: a match is kept when none
/// of its tokens belongs to a match kept before it.
pub(super) fn keep_non_overlapping(
    greedy: Greedy,
    mut matches: Vec<TokenMatch>,
) -> Vec<TokenMatch> {
    // The matches of one rule have distinct spans, so each key orders them completely.
    match greedy {
        Greedy::First => {
            matches.sort_unstable_by_key(|found| (found.start.0, Reverse(found.end.0)));
        }
        Greedy::Longest => matches
            .sort_unstable_by_key(|found| (Reverse(found.end.0 - found.start.0), found.start.0)),
    }
    let tokens = matches.iter().map(|found| found.end.0).max().unwrap_or(0);
    let mut claimed = vec![false; tokens];
    matches.retain(|found| {
        let span = &mut claimed[found.start.0..found.end.0];
        let free = !span.contains(&true);
        if free {
            span.fill(true);
        }
        free
    });
    matches
}
