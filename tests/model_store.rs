use spars::ModelStore;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};
static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "spars-store-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(fs::canonicalize(path).unwrap())
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn invalid_names_and_missing_installation_are_rejected() {
    let temp = Directory::new();
    let store = ModelStore::new(&temp.0);
    for name in ["", "../model", "a/b", "a\\b", ".", "En", "a;bad"] {
        assert!(store.resolve(name).is_err());
    }
    assert!(store
        .resolve("en_core_web_lg")
        .unwrap_err()
        .to_string()
        .contains("spars download en_core_web_lg"));
    assert!(!temp.0.join("spars.json").exists());
}
#[test]
fn registration_is_atomic_and_rejects_traversal() {
    let temp = Directory::new();
    let store = ModelStore::new(&temp.0);
    let installed = temp.0.join("release-one");
    fs::create_dir(&installed).unwrap();
    store.register("en_core_web_lg", &installed).unwrap();
    assert_eq!(store.resolve("en_core_web_lg").unwrap(), installed);
    store.register("en_core_web_lg", &installed).unwrap();
    let outside = Directory::new();
    assert!(store.register("en_core_web_md", &outside.0).is_err());
    fs::write(temp.0.join(".spars/en_core_web_lg"), "../outside").unwrap();
    assert!(store.resolve("en_core_web_lg").is_err());
}
#[test]
fn oversized_and_nonregular_records_are_rejected() {
    let temp = Directory::new();
    let store = ModelStore::new(&temp.0);
    fs::create_dir(temp.0.join(".spars")).unwrap();
    let record = temp.0.join(".spars/en_core_web_lg");
    fs::write(&record, "x".repeat(4097)).unwrap();
    assert!(store.resolve("en_core_web_lg").is_err());
    fs::remove_file(&record).unwrap();
    fs::create_dir(&record).unwrap();
    assert!(store.resolve("en_core_web_lg").is_err());
}
#[cfg(unix)]
#[test]
fn selection_symlinks_cannot_redirect_loading() {
    use std::os::unix::fs::symlink;
    let temp = Directory::new();
    let store = ModelStore::new(&temp.0);
    fs::create_dir(temp.0.join("records")).unwrap();
    symlink(temp.0.join("records"), temp.0.join(".spars")).unwrap();
    assert!(store.resolve("en_core_web_lg").is_err());
    fs::remove_file(temp.0.join(".spars")).unwrap();
    fs::create_dir(temp.0.join(".spars")).unwrap();
    fs::write(temp.0.join("record"), "release").unwrap();
    symlink(temp.0.join("record"), temp.0.join(".spars/en_core_web_lg")).unwrap();
    assert!(store.resolve("en_core_web_lg").is_err());
}

#[cfg(unix)]
#[test]
fn selected_directory_cannot_escape_store() {
    let temp = Directory::new();
    let outside = Directory::new();
    fs::create_dir(temp.0.join(".spars")).unwrap();
    fs::write(temp.0.join(".spars/en_core_web_sm"), "escape\n").unwrap();
    std::os::unix::fs::symlink(&outside.0, temp.0.join("escape")).unwrap();
    assert!(ModelStore::new(&temp.0).resolve("en_core_web_sm").is_err());
}
