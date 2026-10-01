use super::*;

// Compiled conditions with the pattern-level LENGTH table they index into.
struct Compiled {
    conditions: CompiledConditions,
    lengths: Vec<LengthChecks>,
}
impl Compiled {
    fn matches(&self, values: &TokenValues<'_>, index: usize) -> Result<bool> {
        self.conditions
            .lexical
            .matches(values, index, &self.lengths)
    }
}
fn compile(constraints: &[TokenConstraint]) -> Result<Compiled> {
    let mut lengths = Vec::new();
    compile_conditions(constraints, &mut lengths).map(|conditions| Compiled {
        conditions,
        lengths,
    })
}
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
    let values = TokenValues::new(
        &doc,
        Needs {
            lower: compiled.attribute() == TokenAttribute::Lower,
            ..Needs::default()
        },
        None,
    )
    .unwrap();
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
        let values = TokenValues::new(&doc, Needs::default(), None).unwrap();
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
        let values = TokenValues::new(
            &doc,
            Needs {
                lower: compiled.attribute() == TokenAttribute::Lower,
                ..Needs::default()
            },
            None,
        )
        .unwrap();
        let token = doc.token(TokenIndex(index)).unwrap();
        let cached = compiled.matches(token, &values).unwrap();
        let empty = TokenValues::new(&doc, Needs::default(), None).unwrap();
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
    let values = TokenValues::new(
        &doc,
        Needs {
            lower: true,
            ..Needs::default()
        },
        None,
    )
    .unwrap();
    assert_eq!(values.lower, ["ος", "the"]);
    assert!(TokenValues::new(&doc, Needs::default(), None)
        .unwrap()
        .lower
        .is_empty());
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
#[test]
fn malformed_lexical_flag_conditions_are_rejected() {
    for raw in [
        r#"{"attribute":"is_alpha","predicate":{"kind":"flag","value":1}}"#,
        r#"{"attribute":"is_alpha","predicate":{"kind":"flag","value":"true"}}"#,
        r#"{"attribute":"is_alpha","predicate":{"kind":"flag","value":null}}"#,
        r#"{"attribute":"is_alpha","predicate":{"kind":"flag"}}"#,
        r#"{"attribute":"IS_ALPHA","predicate":{"kind":"flag","value":true}}"#,
        r#"{"attribute":"is_upper","predicate":{"kind":"flag","value":true}}"#,
    ] {
        assert!(
            serde_json::from_str::<TokenConstraint>(raw).is_err(),
            "{raw}"
        );
    }
    for attribute in FLAG_ATTRIBUTES {
        let parsed: TokenConstraint = serde_json::from_value(serde_json::json!({
            "attribute": attribute, "predicate": {"kind": "flag", "value": false}
        }))
        .unwrap();
        assert_eq!(parsed.attribute, attribute);
        let compiled = compile(std::slice::from_ref(&parsed)).unwrap();
        assert!(compiled.conditions.constraints.is_empty());
        assert_eq!(
            compiled.conditions.lexical.mask(),
            attribute.flag_bit().unwrap()
        );
        for predicate in [
            Predicate::Equals {
                value: "true".into(),
            },
            Predicate::In {
                values: vec!["true".into()],
            },
            Predicate::NotIn { values: vec![] },
        ] {
            assert!(compile(&[TokenConstraint {
                attribute,
                predicate
            }])
            .is_err());
        }
    }
    for attribute in [
        TokenAttribute::Text,
        TokenAttribute::Lower,
        TokenAttribute::Morphology,
    ] {
        assert!(compile(&[TokenConstraint {
            attribute,
            predicate: Predicate::Flag { value: true }
        }])
        .is_err());
    }
}
#[test]
fn flag_tests_compare_every_required_bit_and_need_a_lexicon() {
    let doc = doc();
    let mask = TokenAttribute::LikeNum.flag_bit().unwrap();
    assert!(TokenValues::new(
        &doc,
        Needs {
            flags: mask,
            ..Needs::default()
        },
        None
    )
    .is_err());
    let flag = |attribute: TokenAttribute, value: bool| TokenConstraint {
        attribute,
        predicate: Predicate::Flag { value },
    };
    let test = |constraints: &[TokenConstraint]| compile(constraints).unwrap();
    let alpha = TokenAttribute::IsAlpha.flag_bit().unwrap();
    // Token 0 is alphabetic only; token 1 is a number only; token 2 has neither flag.
    let values = TokenValues {
        lower: Vec::new(),
        flags: vec![alpha, mask, 0],
        lengths: Vec::new(),
    };
    let check = |flags: Compiled| {
        (0..3)
            .map(|index| flags.matches(&values, index).unwrap())
            .collect::<Vec<_>>()
    };
    assert_eq!(check(test(&[])), [true, true, true]);
    assert_eq!(
        check(test(&[flag(TokenAttribute::IsAlpha, true)])),
        [true, false, false]
    );
    assert_eq!(
        check(test(&[flag(TokenAttribute::IsAlpha, false)])),
        [false, true, true]
    );
    assert_eq!(
        check(test(&[
            flag(TokenAttribute::IsAlpha, false),
            flag(TokenAttribute::LikeNum, true)
        ])),
        [false, true, false]
    );
    assert_eq!(
        check(test(&[
            flag(TokenAttribute::LikeNum, true),
            flag(TokenAttribute::LikeNum, false)
        ])),
        [false, false, false]
    );
    let empty = TokenValues::new(&doc, Needs::default(), None).unwrap();
    assert!(test(&[flag(TokenAttribute::IsAlpha, true)])
        .matches(&empty, 0)
        .is_err());
}
#[test]
fn length_counts_code_points_and_compares_like_spacy() {
    // Code points: "a" 1, decomposed "é" 2, the ZWJ emoji sequence 4, "𐐀" 1.
    let words = [
        "a",
        "e\u{301}",
        "\u{1f469}\u{1f3fd}\u{200d}\u{1f4bb}",
        "\u{10400}",
    ];
    let text = words.join(" ");
    let mut tokens = Vec::new();
    let (mut start, mut idx) = (0, 0);
    for word in words {
        tokens.push(Token::new(start, start + word.len(), idx, word.into()));
        start += word.len() + 1;
        idx += word.chars().count() + 1;
    }
    let doc = Doc {
        text,
        tokens,
        entities: None,
        sentences: None,
        noun_chunks: None,
        tensor: Vec::new(),
        dependency_index: Default::default(),
    };
    let values = TokenValues::new(
        &doc,
        Needs {
            length: true,
            ..Needs::default()
        },
        None,
    )
    .unwrap();
    assert_eq!(values.lengths, [1, 2, 4, 1]);
    let length = |predicate: Predicate| TokenConstraint {
        attribute: TokenAttribute::Length,
        predicate,
    };
    let compare = |operator, value: f64| {
        length(Predicate::Compare {
            operator,
            value: FiniteNumber::new(value).unwrap(),
        })
    };
    let check = |constraints: &[TokenConstraint]| {
        let test = compile(constraints).unwrap();
        (0..4)
            .map(|index| test.matches(&values, index).unwrap())
            .collect::<Vec<_>>()
    };
    assert_eq!(
        check(&[compare(Comparison::Equal, 2.0)]),
        [false, true, false, false]
    );
    assert_eq!(
        check(&[compare(Comparison::NotEqual, 1.0)]),
        [false, true, true, false]
    );
    assert_eq!(
        check(&[compare(Comparison::Greater, 2.5)]),
        [false, false, true, false]
    );
    assert_eq!(
        check(&[compare(Comparison::GreaterOrEqual, 2.0)]),
        [false, true, true, false]
    );
    assert_eq!(
        check(&[compare(Comparison::Less, 2.0)]),
        [true, false, false, true]
    );
    assert_eq!(
        check(&[compare(Comparison::LessOrEqual, 1.0)]),
        [true, false, false, true]
    );
    assert_eq!(
        check(&[
            compare(Comparison::GreaterOrEqual, 2.0),
            compare(Comparison::Less, 4.0)
        ]),
        [false, true, false, false]
    );
    assert_eq!(
        check(&[length(Predicate::InIntegers {
            values: vec![-1, 4]
        })]),
        [false, false, true, false]
    );
    assert_eq!(
        check(&[length(Predicate::NotInIntegers { values: vec![1] })]),
        [false, true, true, false]
    );
    assert_eq!(
        check(&[length(Predicate::InIntegers { values: vec![] })]),
        [false; 4]
    );
    // Fractional values must not be truncated or rounded.
    assert_eq!(check(&[compare(Comparison::Equal, 2.5)]), [false; 4]);
    assert_eq!(
        check(&[compare(Comparison::GreaterOrEqual, 2.5)]),
        [false, false, true, false]
    );
    assert_eq!(
        check(&[compare(Comparison::Less, 2.5)]),
        [true, true, false, true]
    );
    assert_eq!(check(&[compare(Comparison::LessOrEqual, -0.5)]), [false; 4]);
    assert_eq!(
        check(&[compare(Comparison::NotEqual, 2.0)]),
        [true, false, true, true]
    );
    assert_eq!(
        check(&[length(Predicate::InIntegers { values: vec![2, 2] })]),
        [false, true, false, false]
    );
    assert_eq!(
        check(&[length(Predicate::NotInIntegers { values: vec![-1] })]),
        [true; 4]
    );
    // Flags and lengths on one item must both hold.
    let alpha = TokenAttribute::IsAlpha.flag_bit().unwrap();
    let mixed = TokenValues {
        lower: Vec::new(),
        flags: vec![alpha, alpha, 0],
        lengths: vec![1, 4, 4],
    };
    let both = compile(&[
        TokenConstraint {
            attribute: TokenAttribute::IsAlpha,
            predicate: Predicate::Flag { value: true },
        },
        compare(Comparison::GreaterOrEqual, 4.0),
    ])
    .unwrap();
    assert_eq!(
        (0..3)
            .map(|index| both.matches(&mixed, index).unwrap())
            .collect::<Vec<_>>(),
        [false, true, false]
    );
    let without = TokenValues::new(&doc, Needs::default(), None).unwrap();
    assert!(compile(&[compare(Comparison::Equal, 1.0)])
        .unwrap()
        .matches(&without, 0)
        .is_err());
}
#[test]
fn malformed_length_conditions_are_rejected() {
    assert!(FiniteNumber::new(f64::NAN).is_none());
    assert!(FiniteNumber::new(f64::INFINITY).is_none());
    for raw in [
        r#"{"attribute":"length","predicate":{"kind":"compare","operator":"=","value":1}}"#,
        r#"{"attribute":"length","predicate":{"kind":"compare","operator":"==","value":"1"}}"#,
        r#"{"attribute":"length","predicate":{"kind":"compare","operator":"==","value":true}}"#,
        r#"{"attribute":"length","predicate":{"kind":"compare","operator":"==","value":null}}"#,
        r#"{"attribute":"length","predicate":{"kind":"compare","value":1}}"#,
        r#"{"attribute":"length","predicate":{"kind":"in_integers","values":[1.5]}}"#,
        r#"{"attribute":"length","predicate":{"kind":"in_integers","values":["1"]}}"#,
        r#"{"attribute":"length","predicate":{"kind":"in_integers","values":[true]}}"#,
        r#"{"attribute":"length","predicate":{"kind":"in_integers","values":[1e30]}}"#,
        r#"{"attribute":"LENGTH","predicate":{"kind":"compare","operator":"==","value":1}}"#,
        r#"{"attribute":"length","predicate":{"kind":"compare","operator":"==","value":1,"values":[1]}}"#,
    ] {
        assert!(
            serde_json::from_str::<TokenConstraint>(raw).is_err(),
            "{raw}"
        );
    }
    let parsed: TokenConstraint = serde_json::from_str(
        r#"{"attribute":"length","predicate":{"kind":"compare","operator":">","value":2.5}}"#,
    )
    .unwrap();
    assert_eq!(
        parsed.predicate,
        Predicate::Compare {
            operator: Comparison::Greater,
            value: FiniteNumber::new(2.5).unwrap()
        }
    );
    for predicate in [
        Predicate::Equals { value: "3".into() },
        Predicate::In {
            values: vec!["3".into()],
        },
        Predicate::Flag { value: true },
        Predicate::MorphSuperset { values: vec![] },
    ] {
        assert!(matches!(
            compile(&[TokenConstraint {
                attribute: TokenAttribute::Length,
                predicate
            }]),
            Err(Error::Pattern(_))
        ));
    }
    for attribute in [
        TokenAttribute::Text,
        TokenAttribute::Lower,
        TokenAttribute::IsAlpha,
    ] {
        assert!(matches!(
            compile(&[TokenConstraint {
                attribute,
                predicate: Predicate::InIntegers { values: vec![1] }
            }]),
            Err(Error::Pattern(_))
        ));
    }
}
