use super::*;
use crate::{Token, TokenIndex};

fn doc() -> Doc {
    let mut token = Token::new(0, 1, 0, "x".into());
    token.morphology = Some("Case=Acc,Nom|Number=Sing".into());
    Doc {
        text: "x".into(),
        tokens: vec![token],
        entities: None,
        sentences: None,
        noun_chunks: None,
        tensor: Vec::new(),
        dependency_index: Default::default(),
    }
}
fn check(predicate: Predicate) -> bool {
    let doc = doc();
    let compiled = TokenConstraint {
        attribute: TokenAttribute::Morphology,
        predicate,
    }
    .compile()
    .unwrap();
    compiled.validate_document(&doc).unwrap();
    let values = TokenValues::new(&doc, compiled.attribute() == TokenAttribute::Lower).unwrap();
    compiled
        .matches(doc.token(TokenIndex(0)).unwrap(), &values)
        .unwrap()
}
#[test]
fn morphology_equality_sets_and_atomic_features_have_distinct_semantics() {
    assert!(check(Predicate::Equals {
        value: "Case=Acc,Nom|Number=Sing".into()
    }));
    assert!(!check(Predicate::Equals {
        value: "Number=Sing|Case=Nom,Acc".into()
    }));
    assert!(check(Predicate::In {
        values: vec!["Number=Sing|Case=Nom,Acc".into()]
    }));
    assert!(!check(Predicate::NotIn {
        values: vec!["Number=Sing|Case=Nom,Acc".into()]
    }));
    assert!(check(Predicate::MorphSuperset {
        values: vec!["Case=Nom".into(), "Number=Sing".into()]
    }));
    assert!(!check(Predicate::MorphSuperset {
        values: vec!["Case=Acc,Nom".into()]
    }));
    assert!(check(Predicate::MorphIntersects {
        values: vec!["Case=Dat".into(), "Case=Nom".into()]
    }));
    assert!(check(Predicate::MorphSuperset { values: vec![] }));
    assert!(!check(Predicate::MorphIntersects { values: vec![] }));
}
#[test]
fn malformed_constraints_and_missing_annotations_are_explicit() {
    for value in [
        "Case",
        "=Nom",
        "Case=",
        "Case=Nom,",
        "Case=Nom=Acc",
        "Case=Nom Acc",
    ] {
        assert!(TokenConstraint {
            attribute: TokenAttribute::Morphology,
            predicate: Predicate::In {
                values: vec![value.into()]
            }
        }
        .compile()
        .is_err());
    }
    assert!(TokenConstraint {
        attribute: TokenAttribute::Text,
        predicate: Predicate::MorphSuperset { values: vec![] }
    }
    .compile()
    .is_err());
    let constraint = TokenConstraint {
        attribute: TokenAttribute::Lemma,
        predicate: Predicate::Equals { value: "x".into() },
    }
    .compile()
    .unwrap();
    assert!(matches!(
        constraint.validate_document(&doc()),
        Err(Error::MissingAnnotation("lemma"))
    ));
    assert!(serde_json::from_str::<TokenConstraint>(
        r#"{"attribute":"unknown","predicate":{"kind":"equals","value":"x"}}"#
    )
    .is_err());
    assert!(serde_json::from_str::<Predicate>(r#"{"kind":"regex","value":"x"}"#).is_err());
}

#[test]
fn upstream_morphology_duplicate_and_empty_key_rules() {
    assert_eq!(normalize_morph("Case=Acc|Case=Nom").unwrap(), "Case=Nom");
    assert_eq!(normalize_morph("Case=Nom,Nom").unwrap(), "Case=Nom,Nom");
    assert_eq!(normalize_morph("pos=noun").unwrap(), "POS=NOUN");
    assert_eq!(
        normalize_morph("pos=noun|POS=VERB|pos=adj").unwrap(),
        "POS=VERB"
    );
    let mut doc = doc();
    doc.tokens[0].morphology = Some(String::new());
    for (value, expected) in [("", false), ("_", true)] {
        let condition = TokenConstraint {
            attribute: TokenAttribute::Morphology,
            predicate: Predicate::Equals {
                value: value.into(),
            },
        }
        .compile()
        .unwrap();
        let values = TokenValues::new(&doc, false).unwrap();
        assert_eq!(
            condition
                .matches(doc.token(TokenIndex(0)).unwrap(), &values)
                .unwrap(),
            expected
        );
    }
}
#[test]
fn lower_needs_no_annotations_and_compares_pattern_values_as_written() {
    let text = "ΟΣ The";
    let doc = Doc {
        text: text.into(),
        tokens: vec![
            Token::new(0, 4, 0, "ΟΣ".into()),
            Token::new(5, 8, 3, "The".into()),
        ],
        entities: None,
        sentences: None,
        noun_chunks: None,
        tensor: Vec::new(),
        dependency_index: Default::default(),
    };
    let check = |predicate: Predicate, index: usize| {
        let compiled = TokenConstraint {
            attribute: TokenAttribute::Lower,
            predicate,
        }
        .compile()
        .unwrap();
        compiled.validate_document(&doc).unwrap();
        let values = TokenValues::new(&doc, compiled.attribute() == TokenAttribute::Lower).unwrap();
        let token = doc.token(TokenIndex(index)).unwrap();
        let cached = compiled.matches(token, &values).unwrap();
        let empty = TokenValues::new(&doc, false).unwrap();
        assert_eq!(compiled.matches(token, &empty).unwrap(), cached);
        cached
    };
    let equals = |value: &str| Predicate::Equals {
        value: value.into(),
    };
    assert!(check(equals("ος"), 0));
    assert!(!check(equals("οσ"), 0));
    assert!(!check(equals("ΟΣ"), 0));
    assert!(check(equals("the"), 1));
    assert!(!check(equals("The"), 1));
    assert!(check(
        Predicate::In {
            values: vec!["ος".into(), "x".into()]
        },
        0
    ));
    assert!(!check(
        Predicate::NotIn {
            values: vec!["the".into()]
        },
        1
    ));
    assert!(check(Predicate::NotIn { values: vec![] }, 1));
}
#[test]
fn malformed_lower_conditions_are_rejected() {
    for predicate in [
        Predicate::MorphSuperset {
            values: vec!["the".into()],
        },
        Predicate::MorphIntersects { values: vec![] },
    ] {
        assert!(TokenConstraint {
            attribute: TokenAttribute::Lower,
            predicate,
        }
        .compile()
        .is_err());
    }
    for raw in [
        r#"{"attribute":"LOWER","predicate":{"kind":"equals","value":"a"}}"#,
        r#"{"attribute":"lowercase","predicate":{"kind":"equals","value":"a"}}"#,
        r#"{"attribute":"lower","predicate":{"kind":"equals","value":1}}"#,
        r#"{"attribute":"lower","predicate":{"kind":"equals","value":true}}"#,
        r#"{"attribute":"lower","predicate":{"kind":"in","values":"a"}}"#,
        r#"{"attribute":"lower","predicate":{"kind":"equals","value":"a","extra":1}}"#,
    ] {
        assert!(
            serde_json::from_str::<TokenConstraint>(raw).is_err(),
            "{raw}"
        );
    }
    let parsed: TokenConstraint =
        serde_json::from_str(r#"{"attribute":"lower","predicate":{"kind":"equals","value":"a"}}"#)
            .unwrap();
    assert_eq!(parsed.attribute, TokenAttribute::Lower);
}
#[test]
fn borrowed_lowercase_fast_path_matches_the_pinned_resource() {
    let ascii: String = (0u8..128).map(char::from).collect();
    let samples = [
        "",
        "the",
        "The",
        "THE",
        "123",
        ascii.as_str(),
        "ΟΣ",
        "AΣ\u{301}",
        "İ",
        "straße",
        "\u{212a}",
    ];
    for sample in samples {
        let fast = lower(sample);
        assert_eq!(fast, crate::unicode_lower::lower(sample), "{sample:?}");
        let borrowed = matches!(fast, Cow::Borrowed(_));
        let needs_change = !sample.is_ascii() || sample.bytes().any(|b| b.is_ascii_uppercase());
        assert_eq!(borrowed, !needs_change, "{sample:?}");
    }
}
#[test]
fn lower_set_values_are_not_lowercased_and_cache_is_filled_only_when_needed() {
    let doc = Doc {
        text: "ΟΣ The".into(),
        tokens: vec![
            Token::new(0, 4, 0, "ΟΣ".into()),
            Token::new(5, 8, 3, "The".into()),
        ],
        entities: None,
        sentences: None,
        noun_chunks: None,
        tensor: Vec::new(),
        dependency_index: Default::default(),
    };
    let values = TokenValues::new(&doc, true).unwrap();
    assert_eq!(values.lower, ["ος", "the"]);
    assert!(TokenValues::new(&doc, false).unwrap().lower.is_empty());
    let check = |predicate: Predicate, index: usize| {
        TokenConstraint {
            attribute: TokenAttribute::Lower,
            predicate,
        }
        .compile()
        .unwrap()
        .matches(doc.token(TokenIndex(index)).unwrap(), &values)
        .unwrap()
    };
    let uppercase = || vec!["THE".into(), "ΟΣ".into()];
    assert!(!check(
        Predicate::In {
            values: uppercase()
        },
        0
    ));
    assert!(!check(
        Predicate::In {
            values: uppercase()
        },
        1
    ));
    assert!(check(
        Predicate::NotIn {
            values: uppercase()
        },
        0
    ));
    assert!(check(
        Predicate::NotIn {
            values: uppercase()
        },
        1
    ));
}
