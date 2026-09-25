use crate::{invalid, Digest, Identity, ModelName, Result, Version};
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
    #[serde(skip, default = "default_bundle")]
    pub bundle: &'static Bundle,
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

pub(crate) struct Bundle {
    pub recipe: &'static [u8],
    pub template: &'static [u8],
    source_lock: &'static [u8],
}
static MEDIUM: Bundle = Bundle {
    recipe: RECIPE,
    template: TEMPLATE,
    source_lock: include_bytes!("../resources/source-lock.json"),
};
macro_rules! bundle {
    ($path:literal) => {
        Bundle {
            recipe: include_bytes!(concat!($path, "/recipe.json")),
            template: include_bytes!(concat!($path, "/manifest-template.json")),
            source_lock: include_bytes!(concat!($path, "/source-lock.json")),
        }
    };
}
static BUNDLES: &[&Bundle] = &[
    &MEDIUM,
    &bundle!("../models/en_core_web_sm"),
    &bundle!("../models/en_core_web_lg"),
];
fn default_bundle() -> &'static Bundle {
    &MEDIUM
}
impl Bundle {
    pub fn resource(&self, name: &str) -> Result<&'static [u8]> {
        match name {
            "manifest-template.json" => Ok(self.template),
            "source-lock.json" => Ok(self.source_lock),
            "spacy-MIT.txt" => Ok(include_bytes!("../resources/spacy-MIT.txt")),
            "thinc-MIT.txt" => Ok(include_bytes!("../resources/thinc-MIT.txt")),
            "Unicode.txt" => Ok(include_bytes!("../resources/Unicode.txt")),
            _ => Err(invalid("unknown recipe resource")),
        }
    }
    fn load(&'static self) -> Result<Recipe> {
        let mut recipe: Recipe = serde_json::from_slice(self.recipe)?;
        recipe.bundle = self;
        let release = crate::catalog::find(&recipe.identity.model, recipe.identity.model_version)?;
        let fields: BTreeMap<String, serde::de::IgnoredAny> =
            serde_json::from_slice(self.template)?;
        if recipe.identity.format_version.get() != release.format_version
            || recipe.identity.wheel_sha256 != release.wheel_sha256
            || recipe.sources.is_empty()
            || recipe.field_sources.len() != fields.len() + 2
            || Digest::of(self.template) != recipe.template_sha256
        {
            return Err(invalid("unsupported or damaged embedded recipe"));
        }
        for (name, expected) in &recipe.resources {
            if Digest::of(self.resource(name)?) != *expected {
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
}
impl Recipe {
    pub fn prefix(&self) -> String {
        format!(
            "{0}/{0}-{1}/",
            self.identity.model, self.identity.model_version
        )
    }
}
pub(crate) fn select(model: &ModelName, version: Version) -> Result<Recipe> {
    #[derive(Deserialize)]
    struct Header {
        identity: Identity,
    }
    for bundle in BUNDLES {
        let header: Header = serde_json::from_slice(bundle.recipe)?;
        if &header.identity.model == model && header.identity.model_version == version {
            return bundle.load();
        }
    }
    Err(invalid(format!(
        "unsupported model release {model} {version}; no conversion recipe"
    )))
}
#[cfg(test)]
pub(crate) fn load() -> Result<Recipe> {
    MEDIUM.load()
}
#[cfg(test)]
pub(crate) fn resource(name: &str) -> Result<&'static [u8]> {
    MEDIUM.resource(name)
}

/// Verify the original resource inventory without bundling its obsolete notice.
pub(crate) fn for_receipt(identity: &Identity, digest: &Digest) -> Result<Recipe> {
    let mut recipe = select(&identity.model, identity.model_version)?;
    if *identity == recipe.identity && *digest == Digest::of(recipe.bundle.recipe) {
        return Ok(recipe);
    }
    // r1-l2 only removes a notice. All model data and other resource bytes are
    // identical to r1-l1; both the old identity and recipe digest must match.
    if recipe.identity.model != ModelName::EnCoreWebMd
        || recipe.identity.resource_revision.get() != 2
    {
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
    BUNDLES
        .iter()
        .map(|bundle| bundle.load().map(|recipe| recipe.identity))
        .collect()
}

#[cfg(test)]
mod tests;
