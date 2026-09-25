//! Shared, offline model discovery for Rust, Node, and the installer.
use crate::{Error, Model, Result};
use std::{
    fs,
    io::{ErrorKind, Read, Write},
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
/// An explicit model directory. Discovery never downloads or changes files.
#[derive(Clone, Debug)]
pub struct ModelStore {
    root: PathBuf,
}
impl ModelStore {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_owned(),
        }
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
    /// Use SPARS_MODEL_DIR, otherwise the operating system's user cache directory.
    /// Relative environment paths are resolved against the current working directory.
    pub fn discover() -> Result<Self> {
        Ok(Self::new(configured_root(
            std::env::current_dir()?,
            |name| std::env::var_os(name),
        )?))
    }
    /// Resolve the explicitly selected installation, without scanning or reading model weights.
    pub fn resolve(&self, name: &str) -> Result<PathBuf> {
        validate_name(name)?;
        let index = self.root.join(".spars");
        let record = index.join(name);
        let not_installed = |error: std::io::Error| {
            if error.kind() == ErrorKind::NotFound {
                missing(&format!("Model {name} is not installed in {}. Run `spars download {name}` with the same SPARS_MODEL_DIR (or --path).", self.root.display()))
            } else {
                error.into()
            }
        };
        if !fs::symlink_metadata(&index)
            .map_err(not_installed)?
            .file_type()
            .is_dir()
        {
            return Err(Error::Model(
                "model selection directory must not be a symlink".into(),
            ));
        }
        let metadata = fs::symlink_metadata(&record).map_err(not_installed)?;
        if !metadata.file_type().is_file() || metadata.len() > 4096 {
            return Err(Error::Model(
                "model selection must be a regular file of at most 4096 bytes".into(),
            ));
        }
        let file = fs::File::open(record)?;
        if !file.metadata()?.is_file() {
            return Err(Error::Model(
                "model selection must be a regular file".into(),
            ));
        }
        let mut selected = String::new();
        file.take(4097).read_to_string(&mut selected)?;
        if selected.len() > 4096 {
            return Err(Error::Model("model selection is too large".into()));
        }
        let directory = Path::new(selected.trim_end_matches('\n'));
        if !single_component(directory)
            || directory
                .as_os_str()
                .to_string_lossy()
                .contains(['\n', '\r'])
        {
            return Err(Error::Model("invalid model selection record".into()));
        }
        let root = fs::canonicalize(&self.root)?;
        let path = fs::canonicalize(root.join(directory))?;
        if path.parent() != Some(root.as_path()) || !path.is_dir() {
            return Err(Error::Model(
                "selected model must be a directory inside the model store".into(),
            ));
        }
        Ok(path)
    }
    /// Select a completed installation; concurrent writes never expose a partial record.
    pub fn register(&self, name: &str, installed: impl AsRef<Path>) -> Result<()> {
        validate_name(name)?;
        let root = fs::canonicalize(&self.root)?;
        let installed = fs::canonicalize(installed)?;
        if installed.parent() != Some(root.as_path()) || !installed.is_dir() {
            return Err(Error::Model(
                "installation must be a direct child of the model store".into(),
            ));
        }
        let directory = installed
            .file_name()
            .ok_or_else(|| Error::Model("installation has no directory name".into()))?;
        let index = root.join(".spars");
        fs::create_dir_all(&index)?;
        if fs::symlink_metadata(&index)?.file_type().is_symlink() {
            return Err(Error::Model(
                "model selection directory must not be a symlink".into(),
            ));
        }
        let directory = directory
            .to_str()
            .ok_or_else(|| Error::Model("installation directory name must be UTF-8".into()))?;
        if directory.contains(['\n', '\r']) {
            return Err(Error::Model("invalid installation directory name".into()));
        }
        atomic_write(&index.join(name), format!("{directory}\n").as_bytes())
    }
    pub fn load(&self, name: &str) -> Result<Model> {
        let model = Model::load_path(self.resolve(name)?)?;
        if model.config.model != name {
            return Err(Error::Model(
                "installed model identity does not match requested name".into(),
            ));
        }
        Ok(model)
    }
}
pub(crate) fn validate_name(name: &str) -> Result<()> {
    if name.is_empty()
        || name.len() > 128
        || !name.as_bytes()[0].is_ascii_lowercase()
        || !name
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_')
    {
        return Err(Error::Model(
            "invalid model name; use lowercase ASCII letters, digits and underscores".into(),
        ));
    }
    Ok(())
}
fn single_component(path: &Path) -> bool {
    let mut components = path.components();
    matches!(components.next(), Some(Component::Normal(_))) && components.next().is_none()
}
fn missing(message: &str) -> Error {
    std::io::Error::new(ErrorKind::NotFound, message).into()
}
fn atomic_write(path: &Path, value: &[u8]) -> Result<()> {
    let temporary = path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(value)?;
        file.sync_all()?;
        fs::rename(&temporary, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

fn configured_root(
    cwd: PathBuf,
    get: impl Fn(&str) -> Option<std::ffi::OsString>,
) -> Result<PathBuf> {
    if let Some(path) = get("SPARS_MODEL_DIR") {
        if path.is_empty() {
            return Err(Error::Model("SPARS_MODEL_DIR must not be empty".into()));
        }
        return Ok(cwd.join(path));
    }
    let cache = if cfg!(target_os = "windows") {
        get("LOCALAPPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        get("HOME").map(|home| PathBuf::from(home).join("Library/Caches"))
    } else {
        get("XDG_CACHE_HOME")
            .filter(|path| !path.is_empty())
            .map(PathBuf::from)
            .or_else(|| {
                get("HOME")
                    .filter(|path| !path.is_empty())
                    .map(|home| PathBuf::from(home).join(".cache"))
            })
    }
    .filter(|path| path.is_absolute())
    .ok_or_else(|| {
        Error::Model("No user cache directory is available; set SPARS_MODEL_DIR".into())
    })?;
    Ok(cache.join("spars/models"))
}
#[cfg(test)]
mod tests;
