use super::*;

#[test]
fn feature_and_tensor_types_reject_unknown_or_mistyped_fields() {
    assert!(serde_json::from_str::<Feature>(r#""UNKNOWN""#).is_err());
    assert!(serde_json::from_str::<TensorSpec>(r#"{"shape":[2,3],"dtype":"F64"}"#).is_err());
    assert!(serde_json::from_str::<TensorSpec>(r#"{"shape":[-1,3],"dtype":"F32"}"#).is_err());
    assert!(serde_json::from_str::<Linear>(r#"{"W":42,"b":"bias"}"#).is_err());
    assert!(
        serde_json::from_str::<Linear>(r#"{"W":"weights","b":"bias","activation":"relu"}"#)
            .is_err()
    );
}

#[test]
fn attribute_patterns_preserve_attribute_value_types() {
    assert!(serde_json::from_str::<TokenPattern>(r#"{"IS_SPACE":"true"}"#).is_err());
    assert!(serde_json::from_str::<TokenPattern>(r#"{"TAG":true}"#).is_err());
    assert!(serde_json::from_str::<TokenPattern>(r#"{"TAG":{"UNSUPPORTED":["NN"]}}"#).is_err());
    assert!(serde_json::from_str::<TokenPattern>(r#"{"TAG":{"IN":[42]}}"#).is_err());
    assert!(
        serde_json::from_str::<TokenPattern>(r#"{"TAG":{"IN":["NN"]},"IS_SPACE":false}"#).is_ok()
    );
}

#[test]
fn action_deserialization_preserves_label_and_kind() {
    let a: Action = serde_json::from_str(r#""L-pobj||prep""#).unwrap();
    assert_eq!(a.kind, ActionKind::Left);
    assert_eq!(a.label, "pobj||prep");
    assert!(serde_json::from_str::<Action>(r#""UNKNOWN""#).is_err());
}

#[test]
fn optional_attributes_allow_absence_but_reject_null() {
    assert!(serde_json::from_str::<TokenPattern>("{}").is_ok());
    for input in [
        r#"{"TAG":null}"#,
        r#"{"LOWER":null}"#,
        r#"{"DEP":null}"#,
        r#"{"IS_SPACE":null}"#,
        r#"{"TAG":{"IN":null}}"#,
        r#"{"TAG":{"NOT_IN":null}}"#,
        r#"{"TAG":{"REGEX":null}}"#,
    ] {
        assert!(
            serde_json::from_str::<TokenPattern>(input).is_err(),
            "{input}"
        );
    }
}

#[test]
fn tokenizer_optional_boolean_rejects_null_but_nullable_resources_remain_valid() {
    let mut json = serde_json::json!({
        "prefix":"", "suffix":"", "infix":"", "url":"",
        "token_match":null, "regex_dialect":"test", "rules":{}
    });
    assert!(serde_json::from_value::<TokenizerConfig>(json.clone()).is_ok());
    json["faster_heuristics"] = serde_json::Value::Null;
    assert!(serde_json::from_value::<TokenizerConfig>(json.clone()).is_err());
    json["faster_heuristics"] = serde_json::Value::Bool(true);
    assert!(serde_json::from_value::<TokenizerConfig>(json).is_ok());
    assert!(serde_json::from_str::<Exception>(r#"{"ORTH":"x","NORM":null}"#).is_ok());
}
