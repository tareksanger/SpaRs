use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use spars::Model;
use std::collections::BTreeMap;

#[test]
#[ignore = "requires official export; mandatory acceptance test"]
fn expanded_official_parity() {
    check("evaluation-v1");
}

#[test]
#[ignore = "requires official export; mandatory acceptance test"]
fn tokenizer_boundary_regressions() {
    check("tokenizer-boundaries-v1");
}

#[test]
#[ignore = "requires official export; mandatory acceptance test"]
fn unseen_postfix_comparison() {
    check("unseen-v1");
}

fn check(split: &str) {
    let fixture: Value =
        serde_json::from_slice(&std::fs::read(format!("fixtures/{split}.expected.json")).unwrap())
            .unwrap();
    let model = Model::load("assets/en_core_web_md-3.8.0").unwrap();
    let cases = fixture["cases"].as_array().unwrap();
    assert!(!cases.is_empty(), "empty parity suite");
    assert_eq!(fixture["model"], "en_core_web_md 3.8.0");
    assert_eq!(
        fixture["versions"],
        json!({"spacy":"3.8.14", "thinc":"8.3.13"})
    );
    let input = std::fs::read(format!("fixtures/{split}.json")).unwrap();
    assert_eq!(
        fixture["input_sha256"],
        format!("{:x}", Sha256::digest(&input))
    );
    let inputs: Value = serde_json::from_slice(&input).unwrap();
    assert_eq!(inputs["cases"].as_array().unwrap().len(), cases.len());
    for (input, expected) in inputs["cases"].as_array().unwrap().iter().zip(cases) {
        for field in ["id", "category", "text"] {
            assert_eq!(input[field], expected[field], "fixture identity {field}");
        }
    }
    let mut differences = Vec::new();
    let mut categories: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    let mut tokens = 0;
    let mut max_vector_error = 0f32;
    for case in cases {
        let count = categories
            .entry(case["category"].as_str().unwrap().to_owned())
            .or_default();
        count.0 += 1;
        tokens += case["tokens"].as_array().unwrap().len();
        let before = differences.len();
        match model.process(case["text"].as_str().unwrap()) {
            Err(error) => differences
                .push(json!({"id":case["id"], "text":case["text"], "error":error.to_string()})),
            Ok(doc) => {
                let actual = serde_json::to_value(&doc).unwrap();
                for field in ["tokens", "entities", "sentences", "noun_chunks"] {
                    if actual[field] != case[field] {
                        differences.push(json!({"id":case["id"], "text":case["text"], "field":field, "actual":actual[field], "expected":case[field]}));
                    }
                }
                let vector = model.document_vector(&doc);
                let expected = case["vector"].as_array().unwrap();
                assert_eq!(vector.len(), expected.len());
                for (i, (a, b)) in vector.iter().zip(expected).enumerate() {
                    let b = b.as_f64().unwrap() as f32;
                    let delta = (a - b).abs();
                    max_vector_error = max_vector_error.max(delta);
                    if !a.is_finite() || !b.is_finite() || delta > 2e-6 + 2e-6 * b.abs() {
                        differences.push(json!({"id":case["id"], "text":case["text"], "field":"vector", "index":i, "actual":a,"expected":b}));
                    }
                }
            }
        }
        if differences.len() == before {
            count.1 += 1;
        }
    }
    std::fs::create_dir_all("reports").unwrap();
    let report = json!({"model":fixture["model"], "versions":fixture["versions"],
        "input_sha256":fixture["input_sha256"], "cases":cases.len(), "tokens":tokens,
        "category_counts":categories.iter().map(|(name,(total,passed))| (name,json!({"total":total,"passed":passed}))).collect::<BTreeMap<_,_>>(),
        "max_vector_absolute_error":max_vector_error, "differences":differences,
        "command":"cargo test --release --test evaluation -- --include-ignored"});
    std::fs::write(
        format!("reports/{split}.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
    assert!(
        differences.is_empty(),
        "{} differences: see reports/{split}.json",
        differences.len()
    );
}
