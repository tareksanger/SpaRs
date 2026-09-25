use super::*;
#[test]
fn names_are_validated_data_not_a_model_enumeration() {
    for name in ["en_core_web_lg", "de_core_news_sm", "future_model_123"] {
        let parsed: ModelName = name.parse().unwrap();
        assert_eq!(parsed.to_string(), name);
        assert_eq!(
            serde_json::from_str::<ModelName>(&serde_json::to_string(&parsed).unwrap()).unwrap(),
            parsed
        );
    }
    for name in [
        "", "../model", "a/b", "UPPER", "a\\b", "a:stream", "a b", "a\n", "1model",
    ] {
        assert!(name.parse::<ModelName>().is_err(), "{name:?}");
    }
    assert!("a".repeat(129).parse::<ModelName>().is_err());
}
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
