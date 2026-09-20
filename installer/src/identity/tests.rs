use super::*;
#[test]
fn versions_and_hashes_are_checked() {
    for bad in ["3.8", "03.8.0", "../3.8.0", "3.8.0-alpha", "3.8.-1"] {
        assert!(bad.parse::<Version>().is_err());
    }
    assert_eq!("3.8.0".parse::<Version>().unwrap().to_string(), "3.8.0");
    assert!(Digest::try_from("f".repeat(63)).is_err());
    assert!(Digest::try_from("F".repeat(64)).is_err());
    assert_eq!(
        Digest::of(b"abc").to_string(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}
