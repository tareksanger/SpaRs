use crate::{invalid, Digest, Identity, Result};
use serde::Deserialize;
use std::collections::BTreeMap;
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum TensorSource {
    Thinc {
        entry: String,
        node: usize,
        parameter: String,
    },
    Npy {
        entry: String,
    },
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Recipe {
    pub identity: Identity,
    pub lookup_sha256: LookupDigests,
    pub sources: BTreeMap<String, TensorSource>,
    pub inputs: BTreeMap<String, Digest>,
    pub lemma_pos: BTreeMap<String, u64>,
    pub vector_keys_entry: String,
    pub lemmas_entry: String,
    pub template_sha256: Digest,
    pub resources: BTreeMap<String, Digest>,
    pub field_sources: BTreeMap<String, String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LookupDigests {
    pub vector_keys: Digest,
    pub lemmas: Digest,
}
pub(crate) const RECIPE: &[u8] = include_bytes!("../resources/recipe.json");
pub(crate) const TEMPLATE: &[u8] = include_bytes!("../resources/manifest-template.json");
pub(crate) fn resource(name: &str) -> Result<&'static [u8]> {
    match name {
        "manifest-template.json" => Ok(TEMPLATE),
        "source-lock.json" => Ok(include_bytes!("../resources/source-lock.json")),
        "spacy-MIT.txt" => Ok(include_bytes!("../resources/spacy-MIT.txt")),
        "thinc-MIT.txt" => Ok(include_bytes!("../resources/thinc-MIT.txt")),
        "Python.txt" => Ok(include_bytes!("../resources/Python.txt")),
        "Unicode.txt" => Ok(include_bytes!("../resources/Unicode.txt")),
        _ => Err(invalid("unknown recipe resource")),
    }
}
pub(crate) fn load() -> Result<Recipe> {
    let recipe: Recipe = serde_json::from_slice(RECIPE)?;
    if recipe.identity.format_version.get() != 1
        || recipe.sources.len() != 69
        || recipe.field_sources.len() != 22
        || Digest::of(TEMPLATE) != recipe.template_sha256
    {
        return Err(invalid("unsupported or damaged embedded recipe"));
    }
    for (name, expected) in &recipe.resources {
        if Digest::of(resource(name)?) != *expected {
            return Err(invalid(format!("embedded resource checksum: {name}")));
        }
    }
    for source in recipe.sources.values() {
        let entry = match source {
            TensorSource::Thinc { entry, .. } | TensorSource::Npy { entry } => entry,
        };
        if !recipe.inputs.contains_key(entry) {
            return Err(invalid("tensor source missing checksum"));
        }
    }
    Ok(recipe)
}
/// Releases explicitly supported by this installer. No remote discovery occurs.
pub fn catalog() -> Result<Vec<Identity>> {
    Ok(vec![load()?.identity])
}
