use super::*;
#[test]
fn lookup_bounds_and_trailing_bytes() {
    assert_eq!(vector_keys(&[0x81, 1, 2], 3).unwrap()[&1], 2);
    assert!(vector_keys(&[0x81, 1, 3], 3).is_err());
    assert!(vector_keys(&[0x80, 0], 3).is_err());
    assert!(lemmas(&[0x80], &BTreeMap::new()).is_err());
}
