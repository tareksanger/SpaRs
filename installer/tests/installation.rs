use fs2::FileExt;
use spars_model::{catalog, install, list, verify, ModelName};
use std::{
    fs::{self, OpenOptions},
    path::PathBuf,
};
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join(format!("integration-{}", std::process::id()));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
#[ignore = "requires official model archive and Python reference export; acceptance runs it"]
fn official_conversion_and_installation_lifecycle() {
    let scratch = Scratch::new();
    let root = scratch.0.join("models");
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let archive = repo.join("assets/en_core_web_md-3.8.0-py3-none-any.whl");
    assert!(archive.is_file(), "official archive required");
    let identity = catalog().unwrap().remove(0);
    let version = identity.model_version;
    fs::create_dir_all(&root).unwrap();
    let stale = root.join(format!(".{}.staging", identity.directory_name()));
    fs::create_dir_all(&stale).unwrap();
    fs::write(stale.join("partial"), b"incomplete").unwrap();
    let lock = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(root.join(".install.lock"))
        .unwrap();
    lock.lock_exclusive().unwrap();
    assert!(matches!(
        install(&root, Some(&archive), ModelName::EnCoreWebMd, version),
        Err(spars_model::Error::Busy)
    ));
    drop(lock);
    let model = install(&root, Some(&archive), ModelName::EnCoreWebMd, version).unwrap();
    assert_eq!(model.identity.resource_revision.get(), 2);
    assert!(!model.path.join("Python.txt").exists());
    assert!(!stale.exists());
    let reference = repo.join("assets/en_core_web_md-3.8.0");
    let actual: serde_json::Value =
        serde_json::from_slice(&fs::read(model.path.join("manifest.json")).unwrap()).unwrap();
    let expected: serde_json::Value =
        serde_json::from_slice(&fs::read(reference.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(actual, expected, "every manifest field must agree");
    assert_eq!(
        fs::read(model.path.join("weights.safetensors")).unwrap(),
        fs::read(reference.join("weights.safetensors")).unwrap(),
        "all tensor bytes must agree"
    );
    assert_eq!(list(&root).unwrap().len(), 1);
    assert_eq!(
        install(
            &root,
            Some(&scratch.0.join("absent.whl")),
            ModelName::EnCoreWebMd,
            version
        )
        .unwrap()
        .path,
        model.path
    );
    let loaded = spars::Model::load(&model.path).unwrap();
    let doc = loaded.process("Alice visited New York.").unwrap();
    assert!(doc.entities().is_some());
    drop(loaded);
    assert!(install(
        &root,
        Some(&archive),
        ModelName::EnCoreWebMd,
        "3.9.0".parse().unwrap()
    )
    .is_err());
    assert!(verify(&model.path).is_ok());
    let bad = scratch.0.join("bad.whl");
    fs::write(&bad, b"broken").unwrap();
    assert!(install(
        &scratch.0.join("failed"),
        Some(&bad),
        ModelName::EnCoreWebMd,
        version
    )
    .is_err());
    assert!(verify(&model.path).is_ok());
    let manifest_path = model.path.join("manifest.json");
    let saved = fs::read(&manifest_path).unwrap();
    fs::write(&manifest_path, b"{}").unwrap();
    assert!(verify(&model.path).is_err());
    fs::write(&manifest_path, &saved).unwrap();
    let receipt_path = model.path.join("installation.json");
    let receipt = fs::read(&receipt_path).unwrap();
    fs::write(&receipt_path, b"{}").unwrap();
    assert!(verify(&model.path).is_err());
    fs::write(&receipt_path, &receipt).unwrap();
    // Even a forged receipt cannot authorize an altered semantic manifest.
    let mut forged_manifest = actual;
    forged_manifest["model"] = serde_json::json!("forged");
    let changed = serde_json::to_vec(&forged_manifest).unwrap();
    fs::write(&manifest_path, &changed).unwrap();
    let mut forged: serde_json::Value = serde_json::from_slice(&receipt).unwrap();
    forged["files"]["manifest.json"]["bytes"] = serde_json::json!(changed.len());
    forged["files"]["manifest.json"]["sha256"] =
        serde_json::json!(spars_model::Digest::of(&changed).to_string());
    fs::write(&receipt_path, serde_json::to_vec(&forged).unwrap()).unwrap();
    assert!(verify(&model.path).is_err());
    fs::write(&manifest_path, saved).unwrap();
    fs::write(&receipt_path, receipt).unwrap();
    #[cfg(unix)]
    {
        let license = model.path.join("LICENSE");
        let original = fs::read(&license).unwrap();
        fs::remove_file(&license).unwrap();
        std::os::unix::fs::symlink(reference.join("LICENSE"), &license).unwrap();
        assert!(verify(&model.path).is_err());
        fs::remove_file(license).unwrap();
        fs::write(model.path.join("LICENSE"), original).unwrap();
    }
    assert!(verify(&model.path).is_ok());
}
