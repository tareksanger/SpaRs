//! Node wrappers for the shared Rust installer and model-directory configuration.
use crate::errors;
use napi::{
    bindgen_prelude::{AsyncTask, Utf16String},
    Env, Result, Task,
};
use napi_derive::napi;
use std::path::PathBuf;
#[napi(object)]
pub struct DownloadOptions {
    pub path: Option<Utf16String>,
    pub archive: Option<Utf16String>,
    pub version: Option<Utf16String>,
}
/// Download an official model explicitly and remember its installation for loadModel(name).
#[napi(strict)]
pub fn download_model(
    name: Utf16String,
    options: Option<DownloadOptions>,
) -> AsyncTask<DownloadTask> {
    AsyncTask::new(DownloadTask {
        request: prepare_download(name, options),
    })
}
fn prepare_download(
    name: Utf16String,
    input: Option<DownloadOptions>,
) -> errors::Result<(String, spars_model::DownloadOptions)> {
    let name = errors::text(&name)?;
    let cwd = std::env::current_dir().map_err(spars::Error::from)?;
    let mut options = spars_model::DownloadOptions::default();
    if let Some(input) = input {
        options.path = input
            .path
            .as_ref()
            .map(|v| errors::text(v).map(PathBuf::from))
            .transpose()?;
        options.archive = input
            .archive
            .as_ref()
            .map(|v| errors::text(v).map(|p| cwd.join(p)))
            .transpose()?;
        options.version = input
            .version
            .as_ref()
            .map(|v| errors::text(v)?.parse().map_err(errors::BindingError::from))
            .transpose()?;
    }
    // Capture environment and relative paths on the calling thread, before queueing work.
    options.path = Some(match options.path {
        Some(path) if !path.as_os_str().is_empty() => cwd.join(path),
        Some(_) => return Err(spars::Error::Model("model path must not be empty".into()).into()),
        None => spars::ModelStore::discover()?.root().to_owned(),
    });
    Ok((name, options))
}
#[napi(object)]
pub struct LoadOptions {
    pub path: Option<Utf16String>,
}

pub(crate) enum LoadSource {
    Directory(PathBuf),
    Named(String, spars::ModelStore),
}
impl LoadSource {
    pub(crate) fn prepare(
        value: &Utf16String,
        options: Option<LoadOptions>,
    ) -> errors::Result<Self> {
        let value = errors::text(value)?;
        let cwd = std::env::current_dir().map_err(spars::Error::from)?;
        if let Some(path) = options.and_then(|options| options.path) {
            let path = errors::text(&path)?;
            if path.is_empty() {
                return Err(spars::Error::Model("model path must not be empty".into()).into());
            }
            return Ok(Self::Named(value, spars::ModelStore::new(cwd.join(path))));
        }
        // Existing explicit directory loading remains supported.
        if value.contains(['/', '\\']) || value == "." || value == ".." {
            return Ok(Self::Directory(cwd.join(value)));
        }
        Ok(Self::Named(value, spars::ModelStore::discover()?))
    }
    pub(crate) fn load(&self) -> errors::Result<spars::Model> {
        match self {
            Self::Directory(path) => spars::Model::load(path),
            Self::Named(name, store) => store.load(name),
        }
        .map_err(Into::into)
    }
}

pub struct DownloadTask {
    request: errors::Result<(String, spars_model::DownloadOptions)>,
}
impl DownloadTask {
    fn download(&self) -> errors::Result<String> {
        let (name, options) = self.request.as_ref().map_err(Clone::clone)?;
        let installed = spars_model::download_model(name, options.clone())?;
        Ok(installed.path.to_string_lossy().into_owned())
    }
}
#[napi]
impl Task for DownloadTask {
    type Output = errors::Result<String>;
    type JsValue = String;
    fn compute(&mut self) -> Result<Self::Output> {
        Ok(self.download())
    }
    fn resolve(&mut self, env: Env, output: Self::Output) -> Result<String> {
        output.map_err(|error| error.into_napi(env))
    }
}
