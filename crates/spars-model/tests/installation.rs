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
            .join("../../target")
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
#[ignore = "requires official sm/lg wheels and exports; acceptance runs it"]
fn distinct_models_install_side_by_side_and_match_official_exports() {
    use sha2::{Digest as _, Sha256};
    use std::io::Read;
    let scratch = Scratch(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target")
            .join(format!("multi-model-{}", std::process::id())),
    );
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let digest = |path: PathBuf| {
        let mut file = fs::File::open(path).unwrap();
        let mut hash = Sha256::new();
        let mut bytes = [0u8; 65536];
        loop {
            let count = file.read(&mut bytes).unwrap();
            if count == 0 {
                break;
            }
            hash.update(&bytes[..count]);
        }
        hash.finalize()
    };
    let identities = catalog().unwrap();
    assert_eq!(identities.len(), 3);
    for identity in identities
        .into_iter()
        .filter(|entry| entry.model != ModelName::EnCoreWebMd)
    {
        let stem = format!("{}-{}", identity.model, identity.model_version);
        let archive = repo.join(format!("assets/{stem}-py3-none-any.whl"));
        let installed = install(
            &scratch.0,
            Some(&archive),
            identity.model.clone(),
            identity.model_version,
        )
        .unwrap();
        assert_eq!(installed.identity, identity);
        assert_eq!(verify(&installed.path).unwrap().identity, identity);
        let reference = repo.join(format!("assets/{stem}"));
        let actual: serde_json::Value =
            serde_json::from_slice(&fs::read(installed.path.join("manifest.json")).unwrap())
                .unwrap();
        let expected: serde_json::Value =
            serde_json::from_slice(&fs::read(reference.join("manifest.json")).unwrap()).unwrap();
        assert_eq!(actual, expected, "{stem} complete manifest");
        assert_eq!(
            digest(installed.path.join("weights.safetensors")),
            digest(reference.join("weights.safetensors")),
            "{stem} tensors"
        );
        let repeated = install(
            &scratch.0,
            Some(&archive),
            identity.model,
            identity.model_version,
        )
        .unwrap();
        assert_eq!(installed.path, repeated.path);
        let model = spars::Model::load(&installed.path).unwrap();
        assert!(!model
            .process("Alice visited London.")
            .unwrap()
            .tokens()
            .is_empty());
    }
    assert_eq!(list(&scratch.0).unwrap().len(), 2);
    let installed = list(&scratch.0).unwrap();
    let small = installed
        .iter()
        .find(|entry| entry.identity.model.to_string() == "en_core_web_sm")
        .unwrap();
    let large = installed
        .iter()
        .find(|entry| entry.identity.model.to_string() == "en_core_web_lg")
        .unwrap();
    let preserved_large = scratch.0.join(".saved-large");
    fs::rename(&large.path, &preserved_large).unwrap();
    fs::rename(&small.path, &large.path).unwrap();
    let error = install(
        &scratch.0,
        None,
        large.identity.model.clone(),
        large.identity.model_version,
    )
    .unwrap_err();
    assert!(error.to_string().contains("different model identity"));
    assert_eq!(verify(&large.path).unwrap().identity, small.identity);
    assert_eq!(verify(&preserved_large).unwrap().identity, large.identity);
}
#[test]
#[ignore = "requires official model archive and Python reference export; acceptance runs it"]
fn official_conversion_and_installation_lifecycle() {
    let scratch = Scratch::new();
    let root = scratch.0.join("models");
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
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
