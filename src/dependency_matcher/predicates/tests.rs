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
    compiled.matches(doc.token(TokenIndex(0)).unwrap()).unwrap()
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
        assert_eq!(
            condition
                .matches(doc.token(TokenIndex(0)).unwrap())
                .unwrap(),
            expected
        );
    }
}
