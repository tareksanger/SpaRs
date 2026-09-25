use super::*;
#[test]
fn explicit_environment_overrides_defaults_without_reading_project_files() {
    let cwd = std::env::current_dir().unwrap();
    let root = configured_root(cwd.clone(), |name| {
        (name == "SPARS_MODEL_DIR").then(|| "custom models".into())
    })
    .unwrap();
    assert_eq!(root, cwd.join("custom models"));
    assert!(
        configured_root(cwd.clone(), |name| (name == "SPARS_MODEL_DIR")
            .then(|| "".into()))
        .is_err()
    );
    assert!(configured_root(cwd, |_| None).is_err());
}
#[test]
fn operating_system_default_is_under_user_cache() {
    let home = std::env::current_dir().unwrap();
    let root = configured_root(home.clone(), |name| match name {
        "HOME" | "LOCALAPPDATA" => Some(home.clone().into_os_string()),
        _ => None,
    })
    .unwrap();
    let expected = if cfg!(target_os = "windows") {
        home
    } else if cfg!(target_os = "macos") {
        home.join("Library/Caches")
    } else {
        home.join(".cache")
    };
    assert_eq!(root, expected.join("spars/models"));
}
