//! Explicit downloads and offline discovery shared by both public front ends.
use crate::{catalog, install, invalid, InstalledModel, ModelName, Result, Version};
use spars::ModelStore;
use std::path::PathBuf;
/// Omitting version selects the single pinned release for this model, never a remote latest release.
#[derive(Clone, Default)]
pub struct DownloadOptions {
    pub path: Option<PathBuf>,
    pub archive: Option<PathBuf>,
    pub version: Option<Version>,
}
/// Download, verify, and select a model for subsequent name-based loads.
/// An explicit path overrides SPARS_MODEL_DIR for this call; it is not persisted.
pub fn download_model(name: &str, options: DownloadOptions) -> Result<InstalledModel> {
    let model: ModelName = name.parse()?;
    let version = match options.version {
        Some(version) => version,
        None => {
            let releases: Vec<_> = catalog::releases()?
                .into_iter()
                .filter(|release| release.model == model)
                .collect();
            if releases.len() != 1 {
                return Err(invalid(
                    "model has no unique pinned release; specify a supported version",
                ));
            }
            releases[0].version
        }
    };
    // Resolve configuration before doing network work. Malformed configuration must not be ignored.
    let store = match &options.path {
        Some(path) => {
            if path.as_os_str().is_empty() {
                return Err(invalid("model path must not be empty"));
            }
            ModelStore::new(std::env::current_dir()?.join(path))
        }
        None => ModelStore::discover()?,
    };
    let installed = install(store.root(), options.archive.as_deref(), model, version)?;
    store.register(name, &installed.path)?;
    Ok(installed)
}
