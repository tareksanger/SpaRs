//! Acquisition metadata is independent of conversion and inference semantics.
use crate::{invalid, Digest, ModelName, Result, Version};
use serde::Deserialize;

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Limits {
    pub archive: u64,
    pub entry: u64,
    pub expanded: u64,
    pub installed_file: u64,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            archive: 128 * 1024 * 1024,
            entry: 128 * 1024 * 1024,
            expanded: 256 * 1024 * 1024,
            installed_file: 128 * 1024 * 1024,
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Release {
    pub model: ModelName,
    pub version: Version,
    pub wheel_sha256: Digest,
    pub url: String,
    pub format_version: u32,
    pub limits: Limits,
}
pub(crate) fn releases() -> Result<Vec<Release>> {
    let entries: Vec<Release> =
        serde_json::from_slice(include_bytes!("../../../models/catalog.json"))?;
    for entry in &entries {
        let limits = entry.limits;
        if limits.archive == 0
            || limits.entry == 0
            || limits.expanded < limits.entry
            || limits.installed_file == 0
            || limits.entry == u64::MAX
            || !entry
                .url
                .starts_with("https://github.com/explosion/spacy-models/releases/download/")
        {
            return Err(invalid("invalid model acquisition record"));
        }
    }
    Ok(entries)
}
pub(crate) fn find(model: &ModelName, version: Version) -> Result<Release> {
    releases()?
        .into_iter()
        .find(|entry| &entry.model == model && entry.version == version)
        .ok_or_else(|| {
            invalid(format!(
                "unsupported model release {model} {version}; no catalog entry"
            ))
        })
}
