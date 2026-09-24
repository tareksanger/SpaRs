use super::*;
use crate::{ModelName, Version};
use std::num::NonZeroU32;

fn original_identity() -> Identity {
    Identity {
        model: ModelName::EnCoreWebMd,
        model_version: Version {
            major: 3,
            minor: 8,
            patch: 0,
        },
        wheel_sha256: Digest::try_from(
            "5e6329fe3fecedb1d1a02c3ea2172ee0fede6cea6e4aefb6a02d832dba78a310".to_owned(),
        )
        .unwrap(),
        recipe_revision: NonZeroU32::MIN,
        resource_revision: NonZeroU32::MIN,
        format_version: NonZeroU32::MIN,
    }
}

fn original_digest() -> Digest {
    Digest::try_from("5b2ade0c8fc4a6c34514083683d584e14b2a18f2e18c33a396ac1ffaf6f3681c".to_owned())
        .unwrap()
}

#[test]
fn current_resources_exclude_development_tool_license() {
    let current = load().unwrap();
    assert_eq!(current.identity.resource_revision.get(), 2);
    assert_eq!(current.resources.len(), 4);
    assert!(!current.resources.contains_key("Python.txt"));
    assert!(resource("Python.txt").is_err());
    assert!(for_receipt(&current.identity, &Digest::of(RECIPE)).is_ok());
    assert!(for_receipt(&current.identity, &original_digest()).is_err());
}

#[test]
fn original_receipt_keeps_its_exact_notice_checksum() {
    let legacy = for_receipt(&original_identity(), &original_digest()).unwrap();
    let expected = Digest::try_from(
        "3b2f81fe21d181c499c59a256c8e1968455d6689d269aa85373bfb6af41da3bf".to_owned(),
    )
    .unwrap();
    assert_eq!(
        crate::integrity::expected_file("Python.txt", &legacy).unwrap(),
        expected
    );
    let mut resources = legacy.resources;
    assert_eq!(resources.remove("Python.txt"), Some(expected));
    assert_eq!(resources, load().unwrap().resources);
    assert!(for_receipt(&original_identity(), &Digest::of(RECIPE)).is_err());
    assert!(for_receipt(&original_identity(), &Digest::of(b"forged")).is_err());
}

#[test]
fn legacy_receipt_cannot_authorize_other_identities() {
    let original = original_identity();
    let mut variants = Vec::new();
    let mut changed = original.clone();
    changed.model_version.patch = 1;
    variants.push(changed);
    let mut changed = original.clone();
    changed.wheel_sha256 = Digest::of(b"other wheel");
    variants.push(changed);
    let mut changed = original.clone();
    changed.recipe_revision = NonZeroU32::new(2).unwrap();
    variants.push(changed);
    let mut changed = original.clone();
    changed.resource_revision = NonZeroU32::new(2).unwrap();
    variants.push(changed);
    let mut changed = original;
    changed.format_version = NonZeroU32::new(2).unwrap();
    variants.push(changed);
    for identity in variants {
        assert!(for_receipt(&identity, &original_digest()).is_err());
    }
}

#[test]
fn model_receipts_cannot_authorize_another_models_recipe() {
    for left in BUNDLES {
        let recipe = left.load().unwrap();
        assert!(for_receipt(&recipe.identity, &Digest::of(left.recipe)).is_ok());
        for right in BUNDLES {
            if left.recipe != right.recipe {
                assert!(for_receipt(&recipe.identity, &Digest::of(right.recipe)).is_err());
            }
        }
    }
}
