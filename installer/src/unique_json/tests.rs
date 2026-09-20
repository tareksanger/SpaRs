use super::*;
#[test]
fn rejects_duplicates_at_every_level() {
    for text in [r#"{"a":1,"a":2}"#, r#"{"a":[{"b":1,"b":2}]}"#] {
        assert!(serde_json::from_str::<UniqueJson>(text).is_err());
    }
    let text = r#"{"a":[null,true,"x",-2,4,1.5]}"#;
    assert_eq!(
        serde_json::from_str::<UniqueJson>(text).unwrap().0,
        serde_json::from_str::<Value>(text).unwrap()
    );
}
