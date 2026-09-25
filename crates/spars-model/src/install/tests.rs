use super::*;
#[test]
fn unsupported_version_leaves_no_directory() {
    let root = std::env::temp_dir().join(format!("spars-unsupported-{}", std::process::id()));
    assert!(install(
        &root,
        None,
        ModelName::EnCoreWebMd,
        "9.0.0".parse().unwrap()
    )
    .is_err());
    assert!(!root.exists());
}
#[test]
fn identities_distinguish_recipes_and_resources() {
    let identity = recipe::load().unwrap().identity;
    let mut other = identity.clone();
    other.recipe_revision = 2.try_into().unwrap();
    assert_ne!(identity.directory_name(), other.directory_name());
    other = identity.clone();
    other.resource_revision = 3.try_into().unwrap();
    assert_ne!(identity.directory_name(), other.directory_name());
}

#[test]
fn required_inventory_follows_the_receipt_resource_revision() {
    let current = recipe::load().unwrap();
    let mut original_identity = current.identity.clone();
    original_identity.resource_revision = std::num::NonZeroU32::MIN;
    let original_digest = Digest::try_from(
        "5b2ade0c8fc4a6c34514083683d584e14b2a18f2e18c33a396ac1ffaf6f3681c".to_owned(),
    )
    .unwrap();
    let original = recipe::for_receipt(&original_identity, &original_digest).unwrap();
    let mut expected = required_files(&current);
    assert!(!expected.iter().any(|name| name == "Python.txt"));
    expected.push("Python.txt".to_owned());
    expected.sort();
    assert_eq!(required_files(&original), expected);
}

#[test]
#[cfg(unix)]
fn dangling_lock_does_not_create_external_file() {
    let root = std::env::temp_dir().join(format!("spars-lock-test-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let external = root.join("must-not-exist");
    std::os::unix::fs::symlink(&external, root.join(".install.lock")).unwrap();
    assert!(install(
        &root,
        None,
        ModelName::EnCoreWebMd,
        "3.8.0".parse().unwrap()
    )
    .is_err());
    assert!(!external.exists());
    fs::remove_dir_all(root).unwrap();
}
