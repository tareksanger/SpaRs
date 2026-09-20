//! Compare installed JSON data with the independently generated reference recipe.
use crate::{
    invalid,
    recipe::{self, Recipe},
    Digest, Result,
};
use serde_json::Value;
use std::{io::Read, path::Path};
pub(crate) fn manifest(path: &Path, recipe: &Recipe) -> Result<()> {
    // JSON is compared only at the serialization boundary. Computation and
    // inference consume the runtime's fully typed, checked model configuration.
    let checked: crate::unique_json::UniqueJson = serde_json::from_slice(&read_manifest(path)?)?;
    let Value::Object(mut actual) = checked.0 else {
        return Err(invalid("manifest must be an object"));
    };
    let expected: serde_json::Map<String, Value> = serde_json::from_slice(recipe::TEMPLATE)?;
    for (name, digest) in [
        ("vector_keys", &recipe.lookup_sha256.vector_keys),
        ("lemmas", &recipe.lookup_sha256.lemmas),
    ] {
        let section = actual
            .remove(name)
            .ok_or_else(|| invalid("missing manifest section"))?;
        if Digest::of(&serde_json::to_vec(&section)?) != *digest {
            return Err(invalid(format!(
                "installed {name} differs from pinned recipe"
            )));
        }
    }
    if actual != expected {
        return Err(invalid("installed manifest differs from pinned recipe"));
    }
    Ok(())
}
pub(crate) fn expected_file(name: &str, recipe: &Recipe) -> Result<Digest> {
    if let Some(digest) = recipe.resources.get(name) {
        return Ok(digest.clone());
    }
    if name == "weights.safetensors" {
        #[derive(serde::Deserialize)]
        struct Weights {
            weights_sha256: Digest,
        }
        return Ok(serde_json::from_slice::<Weights>(recipe::TEMPLATE)?.weights_sha256);
    }
    let entry = format!("en_core_web_md/en_core_web_md-3.8.0/{name}");
    recipe
        .inputs
        .get(&entry)
        .cloned()
        .ok_or_else(|| invalid("unknown installed file"))
}

fn read_manifest(path: &Path) -> Result<Vec<u8>> {
    const LIMIT: u64 = 64 * 1024 * 1024;
    let file = std::fs::File::open(path)?;
    if file.metadata()?.len() > LIMIT {
        return Err(invalid("manifest exceeds 64 MiB limit"));
    }
    let mut bytes = Vec::new();
    file.take(LIMIT + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > LIMIT {
        return Err(invalid("manifest exceeds 64 MiB limit"));
    }
    Ok(bytes)
}
#[cfg(test)]
mod tests;
