use serde::Deserialize;
use sha2::{Digest, Sha256};
use spars::{Model, TokenIndex};
use std::collections::BTreeMap;

#[derive(Deserialize)]
struct Fixture {
    versions: BTreeMap<String, String>,
    model: String,
    input_sha256: String,
    cases: Vec<Case>,
}
#[derive(Deserialize)]
struct Case {
    id: String,
    text: String,
    tokens: Vec<Relations>,
}
#[derive(Deserialize)]
struct Relations {
    index: usize,
    children: Vec<usize>,
    ancestors: Vec<usize>,
    subtree: Vec<usize>,
    sentence: [usize; 2],
}

#[test]
#[ignore = "requires official export and traversal fixture; mandatory CI"]
fn traversal_matches_official_spacy() {
    let fixture: Fixture = serde_json::from_slice(
        &std::fs::read("../../fixtures/traversal-v1.expected.json").unwrap(),
    )
    .unwrap();
    assert_eq!(fixture.model, "en_core_web_md 3.8.0");
    assert_eq!(fixture.versions["spacy"], "3.8.14");
    assert_eq!(fixture.versions["thinc"], "8.3.13");
    assert_eq!(
        fixture.input_sha256,
        Sha256::digest(std::fs::read("../../fixtures/evaluation-v1.expected.json").unwrap())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    );
    assert_eq!(fixture.cases.len(), 98);
    let model = Model::load("../../assets/en_core_web_md-3.8.0").unwrap();
    let mut tokens = 0;
    for case in fixture.cases {
        let doc = model.process(&case.text).unwrap();
        assert_eq!(doc.tokens().len(), case.tokens.len(), "{}", case.id);
        let snapshot = doc.to_json().unwrap();
        for expected in case.tokens {
            let token = doc.token(TokenIndex(expected.index)).unwrap();
            let indices = |iter: Vec<spars::TokenView<'_>>| {
                iter.into_iter().map(|t| t.index().0).collect::<Vec<_>>()
            };
            assert_eq!(
                indices(token.children().unwrap().collect()),
                expected.children,
                "{} token {} children",
                case.id,
                expected.index
            );
            assert_eq!(
                indices(token.ancestors().unwrap().collect()),
                expected.ancestors,
                "{} token {} ancestors",
                case.id,
                expected.index
            );
            assert_eq!(
                indices(token.subtree().unwrap().collect()),
                expected.subtree,
                "{} token {} subtree",
                case.id,
                expected.index
            );
            let sentence = token.sentence().unwrap();
            assert_eq!(
                [sentence.start().0, sentence.end().0],
                expected.sentence,
                "{} token {} sentence",
                case.id,
                expected.index
            );
            tokens += 1;
        }
        assert_eq!(
            doc.to_json().unwrap(),
            snapshot,
            "traversal must not alter snapshots"
        );
    }
    assert_eq!(tokens, 5568);
}
