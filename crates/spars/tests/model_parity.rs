use serde::Deserialize;
use spars::{Doc, Model, Span, Token, TokenIndex};
use std::{collections::BTreeMap, fs};

#[derive(Deserialize)]
struct Fixture {
    model: String,
    versions: BTreeMap<String, String>,
    cases: Vec<Case>,
}
#[derive(Deserialize)]
struct Case {
    id: String,
    text: String,
    tokens: Vec<Token>,
    entities: Vec<Span>,
    sentences: Vec<Span>,
    noun_chunks: Vec<Span>,
    vector: Vec<f32>,
}
#[derive(Deserialize)]
struct VectorFixture {
    vector_model: String,
    vector_cases: Vec<VectorCase>,
}
#[derive(Deserialize)]
struct VectorCase {
    text: String,
    tokens: Vec<Vec<f32>>,
    document: Vec<f32>,
    span: Vec<f32>,
    empty_span: Vec<f32>,
}

#[test]
#[ignore = "requires official sm export; mandatory acceptance test"]
fn contextual_token_and_span_vectors_match_independent_reference() {
    let fixture: VectorFixture = serde_json::from_slice(
        &fs::read("../../fixtures/model-capabilities-v1.expected.json").unwrap(),
    )
    .unwrap();
    assert_eq!(fixture.vector_model, "en_core_web_sm 3.8.0");
    let model = Model::load("../../assets/en_core_web_sm-3.8.0").unwrap();
    assert_eq!(fixture.vector_cases.len(), 4);
    for case in fixture.vector_cases {
        let doc = model.process(&case.text).unwrap();
        assert_eq!(doc.tokens().len(), case.tokens.len());
        for (i, expected) in case.tokens.iter().enumerate() {
            close(
                model
                    .token_vector(doc.token(TokenIndex(i)).unwrap())
                    .unwrap(),
                expected,
                &case.text,
            );
        }
        close(&model.document_vector(&doc), &case.document, &case.text);
        let count = doc.tokens().len();
        close(
            &model
                .span_vector(&doc, TokenIndex(1.min(count)), TokenIndex(3.min(count)))
                .unwrap(),
            &case.span,
            &case.text,
        );
        close(
            &model
                .span_vector(&doc, TokenIndex(0), TokenIndex(0))
                .unwrap(),
            &case.empty_span,
            &case.text,
        );
        assert_eq!(model.similarity(&doc, &doc), 1.0);
        if !doc.tokens().is_empty() {
            assert_eq!(model.similarity(&doc, &model.process("").unwrap()), 0.0);
        }
    }
}
fn close(actual: &[f32], expected: &[f32], context: &str) {
    assert_eq!(actual.len(), expected.len(), "{context}");
    for (i, (a, b)) in actual.iter().zip(expected).enumerate() {
        assert!(
            a.is_finite() && b.is_finite() && (a - b).abs() <= 2e-6 + 2e-6 * b.abs(),
            "{context} vector {i}: {a} != {b}"
        );
    }
}

#[test]
#[ignore = "requires official sm/lg exports; mandatory acceptance test"]
fn official_small_and_large_pipeline_parity() {
    for size in ["sm", "lg"] {
        let model = Model::load(format!("../../assets/en_core_web_{size}-3.8.0")).unwrap();
        let fixture: Fixture = serde_json::from_slice(
            &fs::read(format!("../../fixtures/model-{size}-v1.expected.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(fixture.model, format!("en_core_web_{size} 3.8.0"));
        assert_eq!(fixture.versions["spacy"], "3.8.14");
        assert_eq!(fixture.versions["thinc"], "8.3.13");
        assert_eq!(fixture.cases.len(), 98);
        for case in fixture.cases {
            let doc = model.process(&case.text).unwrap();
            assert_eq!(doc.tokens(), case.tokens, "{size}: {} tokens", case.id);
            assert_eq!(
                doc.entities(),
                Some(case.entities.as_slice()),
                "{size}: {} entities",
                case.id
            );
            assert_eq!(
                doc.sentences(),
                Some(case.sentences.as_slice()),
                "{size}: {} sentences",
                case.id
            );
            assert_eq!(
                doc.noun_chunks(),
                Some(case.noun_chunks.as_slice()),
                "{size}: {} chunks",
                case.id
            );
            close(
                &model.document_vector(&doc),
                &case.vector,
                &format!("{size}: {}", case.id),
            );
            let restored = Doc::from_json(&doc.to_json().unwrap()).unwrap();
            assert_eq!(doc, restored);
            close(&model.document_vector(&restored), &case.vector, &case.id);
        }
    }
}

#[test]
#[ignore = "requires official sm export; mandatory acceptance test"]
fn contextual_vectors_preserve_empty_and_unavailable_states() {
    let model = Model::load("../../assets/en_core_web_sm-3.8.0").unwrap();
    assert!(model.vector("hello").is_none());
    let raw = model.tokenize("hello world").unwrap();
    assert!(model.document_vector(&raw).is_empty());
    assert!(model
        .token_vector(raw.token(TokenIndex(0)).unwrap())
        .is_none());
    let doc = model.process("hello world").unwrap();
    let shared = model.tok2vec(&raw);
    assert_eq!(shared.len(), 2);
    for (i, row) in shared.iter().enumerate() {
        assert_eq!(
            model.token_vector(doc.token(TokenIndex(i)).unwrap()),
            Some(row.as_slice())
        );
    }
    assert_eq!(model.document_vector(&doc).len(), 96);
    assert!(model
        .span_vector(&doc, TokenIndex(1), TokenIndex(1))
        .unwrap()
        .is_empty());
    assert!(model
        .document_vector(&model.process("").unwrap())
        .is_empty());
}

#[test]
fn malformed_contextual_snapshots_are_rejected() {
    let original = serde_json::json!({"format_version":2,"document":{
        "text":"x", "tokens":[{"start":0,"end":1,"idx":0,"whitespace":false,"norm":"x"}],
        "entities":null,"sentences":null,"noun_chunks":null,"tensor":[[1.0,2.0]]}});
    assert!(Doc::from_json(&original.to_string()).is_ok());
    for tensor in [serde_json::json!([[]]), serde_json::json!([[1.0], [2.0]])] {
        let mut malformed = original.clone();
        malformed["document"]["tensor"] = tensor;
        assert!(Doc::from_json(&malformed.to_string())
            .err()
            .unwrap()
            .to_string()
            .contains("invalid document tensor"));
    }
    let mut oversized = original.clone();
    oversized["document"]["tensor"] = serde_json::json!([vec![0.; 4097]]);
    assert!(Doc::from_json(&oversized.to_string()).is_err());
    let mut legacy = original;
    legacy["format_version"] = serde_json::json!(1);
    assert!(Doc::from_json(&legacy.to_string())
        .err()
        .unwrap()
        .to_string()
        .contains("contextual vectors require"));
}
