use crate::{
    archive::{hash_file, valid_name, Wheel},
    convert, download, invalid, recipe, Digest, Error, Identity, ModelName, Result, Version,
};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
};
#[derive(Debug, Serialize)]
pub struct InstalledModel {
    pub path: PathBuf,
    pub identity: Identity,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileRecord {
    bytes: u64,
    sha256: Digest,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    identity: Identity,
    recipe_sha256: Digest,
    files: BTreeMap<String, FileRecord>,
}
struct Stage(PathBuf);
impl Drop for Stage {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn regular(path: &Path) -> Result<()> {
    if !fs::symlink_metadata(path)?.file_type().is_file() {
        return Err(invalid("expected regular installation file"));
    }
    Ok(())
}
fn directory(path: &Path) -> Result<()> {
    if !fs::symlink_metadata(path)?.file_type().is_dir() {
        return Err(invalid("expected installation directory, not a link"));
    }
    Ok(())
}
fn required_files() -> Result<Vec<String>> {
    let mut names: Vec<String> = recipe::load()?
        .resources
        .into_keys()
        .filter(|n| n != "manifest-template.json")
        .collect();
    names.extend(
        [
            "manifest.json",
            "weights.safetensors",
            "LICENSE",
            "LICENSES_SOURCES",
        ]
        .map(String::from),
    );
    names.sort();
    Ok(names)
}
/// Convert an official local archive, or explicitly download it when `archive` is absent.
/// Existing installations are verified and reused, never overwritten.
pub fn install(
    root: &Path,
    archive: Option<&Path>,
    model: ModelName,
    version: Version,
) -> Result<InstalledModel> {
    let recipe = recipe::load()?;
    if recipe.identity.model != model || recipe.identity.model_version != version {
        return Err(invalid(
            "unsupported model release; update the installer compatibility catalog",
        ));
    }
    fs::create_dir_all(root)?;
    directory(root)?;
    let lock_path = root.join(".install.lock");
    match fs::symlink_metadata(&lock_path) {
        Ok(metadata) if !metadata.file_type().is_file() => {
            return Err(invalid("installation lock must be a regular file"))
        }
        Ok(_) => (),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
        Err(error) => return Err(error.into()),
    }
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    let lock = options.open(lock_path)?;
    lock.try_lock_exclusive().map_err(|e| {
        if e.kind() == std::io::ErrorKind::WouldBlock {
            Error::Busy
        } else {
            Error::Io(e)
        }
    })?;
    let destination = root.join(recipe.identity.directory_name());
    if destination.try_exists()? {
        return verify(&destination);
    }
    // The OS lock is released on process exit; a later invocation can clean up
    // the deterministic staging directory left by an interrupted installation.
    let staging = root.join(format!(".{}.staging", recipe.identity.directory_name()));
    if staging.try_exists()? {
        directory(&staging)?;
        fs::remove_dir_all(&staging)?;
    }
    fs::create_dir(&staging)?;
    let stage = Stage(staging);
    let downloaded = stage.0.join("official.whl");
    let archive = match archive {
        Some(path) => path,
        None => {
            download::download(&downloaded, &recipe.identity.wheel_sha256)?;
            &downloaded
        }
    };
    let mut wheel = Wheel::open(archive, &recipe.identity.wheel_sha256)?;
    convert::convert(&mut wheel, &recipe, &stage.0)?;
    drop(wheel);
    crate::integrity::manifest(&stage.0.join("manifest.json"), &recipe)?;
    if downloaded.exists() {
        fs::remove_file(&downloaded)?;
    }
    // Validate every tensor and resource using the same checked native loader
    // used by applications before an installation becomes visible.
    drop(spars::Model::load(&stage.0)?);
    let mut files = BTreeMap::new();
    for name in required_files()? {
        let path = stage.0.join(&name);
        files.insert(
            name,
            FileRecord {
                bytes: fs::metadata(&path)?.len(),
                sha256: hash_file(&path)?,
            },
        );
        File::open(path)?.sync_all()?;
    }
    let receipt = Receipt {
        identity: recipe.identity.clone(),
        recipe_sha256: Digest::of(recipe::RECIPE),
        files,
    };
    let receipt_path = stage.0.join("installation.json");
    fs::write(&receipt_path, serde_json::to_vec_pretty(&receipt)?)?;
    File::open(receipt_path)?.sync_all()?;
    #[cfg(unix)]
    File::open(&stage.0)?.sync_all()?;
    fs::rename(&stage.0, &destination)?;
    #[cfg(unix)]
    File::open(root)?.sync_all()?;
    Ok(InstalledModel {
        path: destination,
        identity: recipe.identity,
    })
}
/// Check installed file integrity and compatibility with this installer.
pub fn verify(path: &Path) -> Result<InstalledModel> {
    directory(path)?;
    let receipt_path = path.join("installation.json");
    regular(&receipt_path)?;
    if fs::metadata(&receipt_path)?.len() > 65536 {
        return Err(invalid("installation receipt too large"));
    }
    let receipt: Receipt = serde_json::from_slice(&fs::read(receipt_path)?)?;
    let recipe = recipe::load()?;
    if receipt.identity != recipe.identity || receipt.recipe_sha256 != Digest::of(recipe::RECIPE) {
        return Err(invalid("unsupported installation recipe"));
    }
    let expected = required_files()?;
    if receipt.files.keys().ne(expected.iter()) {
        return Err(invalid("installation inventory differs from recipe"));
    }
    for (name, record) in &receipt.files {
        valid_name(name)?;
        let file = path.join(name);
        regular(&file)?;
        if name != "manifest.json"
            && record.sha256 != crate::integrity::expected_file(name, &recipe)?
        {
            return Err(invalid("receipt differs from pinned file checksum"));
        }
        if record.bytes > 128 * 1024 * 1024 {
            return Err(invalid("installed file exceeds size limit"));
        }
        if fs::metadata(&file)?.len() != record.bytes || hash_file(&file)? != record.sha256 {
            return Err(invalid(format!("damaged installed file: {name}")));
        }
    }
    crate::integrity::manifest(&path.join("manifest.json"), &recipe)?;
    Ok(InstalledModel {
        path: path.into(),
        identity: receipt.identity,
    })
}
/// List verified installations. A damaged or unsupported installation returns an error.
pub fn list(root: &Path) -> Result<Vec<InstalledModel>> {
    if !root.try_exists()? {
        return Ok(Vec::new());
    }
    directory(root)?;
    let mut paths = fs::read_dir(root)?
        .map(|entry| entry.map(|e| e.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    paths.sort();
    let mut results = Vec::new();
    for path in paths {
        if path
            .file_name()
            .is_some_and(|n| n.to_string_lossy().starts_with('.'))
        {
            continue;
        }
        results.push(verify(&path)?);
    }
    Ok(results)
}
#[cfg(test)]
mod tests;
