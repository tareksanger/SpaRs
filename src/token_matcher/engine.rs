use super::{CompiledPattern, Quantifier, TokenMatch};
use crate::{Doc, Result, TokenIndex};
use std::collections::HashSet;

#[derive(Clone, Copy, Eq, PartialEq, Hash)]
struct State {
    pattern: usize,
    node: usize,
    start: usize,
}

pub(super) fn find(patterns: &[CompiledPattern], doc: &Doc) -> Result<Vec<TokenMatch>> {
    // For short patterns the extra lookup costs more than repeated suffix work.
    // Separate compiled paths keep ordinary matching free of per-state overhead.
    if patterns.iter().any(needs_suffix_cache) {
        run::<true>(patterns, doc)
    } else {
        run::<false>(patterns, doc)
    }
}

fn needs_suffix_cache(pattern: &CompiledPattern) -> bool {
    pattern
        .nodes
        .iter()
        .filter(|node| matches!(node.quantifier, Quantifier::Optional | Quantifier::Star))
        .take(9)
        .count()
        > 8
}

fn run<const CACHE_SUFFIXES: bool>(
    patterns: &[CompiledPattern],
    doc: &Doc,
) -> Result<Vec<TokenMatch>> {
    if patterns.iter().all(|pattern| pattern.nodes.is_empty()) {
        return Ok(Vec::new());
    }
    let branching: Vec<bool> = if CACHE_SUFFIXES {
        patterns.iter().map(needs_suffix_cache).collect()
    } else {
        Vec::new()
    };
    let mut visited = HashSet::new();
    let mut checked = HashSet::new();
    for constraint in patterns.iter().flat_map(|p| &p.constraints).flatten() {
        if checked.insert(constraint.attribute()) {
            constraint.validate_document(doc)?;
        }
    }
    let mut states = Vec::new();
    let mut retained = Vec::new();
    let mut branches = Vec::new();
    let mut unique_states = HashSet::new();
    let mut seen = HashSet::new();
    let mut output = Vec::new();
    let mut cache: Vec<Vec<Option<bool>>> = patterns
        .iter()
        .map(|p| vec![None; p.constraints.len()])
        .collect();
    let mut emit = |pattern: usize, start: usize, end: usize| {
        if start < end && seen.insert((patterns[pattern].name.as_str(), start, end)) {
            output.push(TokenMatch {
                rule: patterns[pattern].name.clone(),
                start: TokenIndex(start),
                end: TokenIndex(end),
            });
        }
    };
    for index in 0..doc.tokens().len() {
        visited.clear();
        for values in &mut cache {
            values.fill(None);
        }
        states.extend(
            (0..patterns.len())
                .filter(|&pattern| !patterns[pattern].nodes.is_empty())
                .map(|pattern| State {
                    pattern,
                    node: 0,
                    start: index,
                }),
        );
        let token = doc.token(TokenIndex(index))?;
        for mut state in states.drain(..) {
            let pattern = &patterns[state.pattern];
            loop {
                // Reaching the same state at this token repeats the same emissions
                // and branches. Keep its first traversal, including optional skips.
                if CACHE_SUFFIXES && branching[state.pattern] && !visited.insert(state) {
                    break;
                }
                let node = &pattern.nodes[state.node];
                let matches = if let Some(value) = cache[state.pattern][node.item] {
                    value
                } else {
                    let mut value = true;
                    for constraint in &pattern.constraints[node.item] {
                        if !constraint.matches(token)? {
                            value = false;
                            break;
                        }
                    }
                    cache[state.pattern][node.item] = Some(value);
                    value
                };
                let final_node = state.node + 1 == pattern.nodes.len();
                match node.quantifier {
                    Quantifier::One | Quantifier::Negated => {
                        let matches = matches != matches!(node.quantifier, Quantifier::Negated);
                        if matches {
                            if final_node {
                                emit(state.pattern, state.start, index + 1);
                            } else {
                                state.node += 1;
                                retained.push(state);
                            }
                        }
                        break;
                    }
                    Quantifier::Star => {
                        if final_node {
                            emit(state.pattern, state.start, index);
                            if matches {
                                retained.push(state);
                            }
                            break;
                        }
                        if matches {
                            branches.push(state);
                        }
                        state.node += 1;
                    }
                    Quantifier::Optional => {
                        if final_node {
                            emit(state.pattern, state.start, index);
                            if matches {
                                emit(state.pattern, state.start, index + 1);
                            }
                            break;
                        }
                        state.node += 1;
                        if matches {
                            branches.push(state);
                        }
                    }
                }
            }
        }
        // Equivalent states have identical future emissions. Retaining their first
        // occurrence preserves match order while avoiding combinatorial path growth.
        unique_states.clear();
        for state in retained.drain(..).chain(branches.drain(..)) {
            if unique_states.insert(state) {
                states.push(state);
            }
        }
    }
    for state in states {
        if patterns[state.pattern].nodes[state.node..]
            .iter()
            .all(|node| matches!(node.quantifier, Quantifier::Star | Quantifier::Optional))
        {
            emit(state.pattern, state.start, doc.tokens().len());
        }
    }
    Ok(output)
}
