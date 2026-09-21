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

/// Verify the original resource inventory without bundling its obsolete notice.
pub(crate) fn for_receipt(identity: &Identity, digest: &Digest) -> Result<Recipe> {
    let mut recipe = load()?;
    if *identity == recipe.identity && *digest == Digest::of(RECIPE) {
        return Ok(recipe);
    }
    // r1-l2 only removes a notice. All model data and other resource bytes are
    // identical to r1-l1; both the old identity and recipe digest must match.
    if recipe.identity.resource_revision.get() != 2 {
        return Err(invalid("unsupported installation recipe"));
    }
    recipe.identity.resource_revision = std::num::NonZeroU32::MIN;
    let legacy_digest = Digest::try_from(
        "5b2ade0c8fc4a6c34514083683d584e14b2a18f2e18c33a396ac1ffaf6f3681c".to_owned(),
    )?;
    if *identity != recipe.identity || *digest != legacy_digest {
        return Err(invalid("unsupported installation recipe"));
    }
    recipe.resources.insert(
        "Python.txt".to_owned(),
        Digest::try_from(
            "3b2f81fe21d181c499c59a256c8e1968455d6689d269aa85373bfb6af41da3bf".to_owned(),
        )?,
    );
    Ok(recipe)
}
/// Releases explicitly supported by this installer. No remote discovery occurs.
pub fn catalog() -> Result<Vec<Identity>> {
    Ok(vec![load()?.identity])
}

#[cfg(test)]
mod tests;
