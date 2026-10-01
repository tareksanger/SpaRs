//! Model-independent Python Unicode 15 `str.lower()`, shared by matcher LOWER
//! conditions and including context-sensitive Greek final sigma. Regenerate with
//! tools/phrase_lower_reference.py; provenance is retained in the independent
//! reference fixture and the repository's Unicode notices.
use serde::Deserialize;
use std::{collections::HashMap, sync::OnceLock};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Resources {
    unicode_version: String,
    lower: HashMap<char, String>,
    cased: Vec<[u32; 2]>,
    case_ignorable: Vec<[u32; 2]>,
}
fn resources() -> &'static Resources {
    static RESOURCES: OnceLock<Resources> = OnceLock::new();
    RESOURCES.get_or_init(|| {
        let data: Resources = serde_json::from_str(include_str!("unicode_lower.json"))
            .expect("verified pinned Unicode lower resource");
        assert_eq!(data.unicode_version, "15.0.0");
        data
    })
}
fn contains(ranges: &[[u32; 2]], character: char) -> bool {
    let point = u32::from(character);
    let index = ranges.partition_point(|range| range[1] < point);
    ranges.get(index).is_some_and(|range| range[0] <= point)
}
pub(crate) fn lower(text: &str) -> String {
    if text.is_ascii() {
        return text.to_ascii_lowercase();
    }
    let data = resources();
    let characters: Vec<char> = text.chars().collect();
    // The nearest non-ignorable character on each side determines final sigma.
    // Two linear passes avoid rescanning arbitrarily long combining sequences.
    let mut following = vec![false; characters.len()];
    let mut next_cased = false;
    for (index, &character) in characters.iter().enumerate().rev() {
        following[index] = next_cased;
        if !contains(&data.case_ignorable, character) {
            next_cased = contains(&data.cased, character);
        }
    }
    let mut previous_cased = false;
    let mut output = String::with_capacity(text.len());
    for (index, &character) in characters.iter().enumerate() {
        if character == 'Σ' && previous_cased && !following[index] {
            output.push('ς');
        } else if let Some(mapped) = data.lower.get(&character) {
            output.push_str(mapped);
        } else {
            output.push(character);
        }
        if !contains(&data.case_ignorable, character) {
            previous_cased = contains(&data.cased, character);
        }
    }
    output
}
#[cfg(test)]
mod tests;
