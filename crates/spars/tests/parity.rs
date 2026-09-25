use serde_json::Value;
use spars::Model;
#[test]
#[ignore = "requires explicit official model export; CI runs with --ignored"]
fn official_development_parity() {
    check("development");
}
#[test]
#[ignore = "requires official model export; mandatory CI"]
fn official_holdout_parity() {
    check("holdout");
}
fn check(split: &str) {
    let m = Model::load("../../assets/en_core_web_md-3.8.0").unwrap();
    let fixture: Value = serde_json::from_str(
        &std::fs::read_to_string(format!("../../fixtures/{split}.expected.json")).unwrap(),
    )
    .unwrap();
    let mut differences = vec![];
    let mut max_error = 0f32;
    for (i, c) in fixture["cases"].as_array().unwrap().iter().enumerate() {
        let text = c["text"].as_str().unwrap();
        let tokenized = m.tokenize(text).unwrap();
        let x = m.tok2vec(&tokenized);
        if let Some(expected) = c["tok2vec"].as_array() {
            assert_eq!(x.len(), expected.len(), "tok2vec row count");
            {
                for (a, b) in x.iter().zip(expected) {
                    assert_eq!(a.len(), b.as_array().unwrap().len(), "tok2vec width");
                    for (a, b) in a.iter().zip(b.as_array().unwrap()) {
                        let delta = (a - b.as_f64().unwrap() as f32).abs();
                        assert!(delta <= 2e-5 + 2e-5 * b.as_f64().unwrap().abs() as f32);
                        max_error = max_error.max(delta)
                    }
                }
            }
        }
        let d = m.process(text).unwrap();
        assert_eq!(d, m.process(text).unwrap(), "repeat {i}");
        let vector = m.document_vector(&d);
        assert_eq!(vector.len(), c["vector"].as_array().unwrap().len());
        for (a, b) in vector.iter().zip(c["vector"].as_array().unwrap()) {
            let b = b.as_f64().unwrap() as f32;
            assert!((a - b).abs() <= 2e-6 + 2e-6 * b.abs());
        }
        let actual = serde_json::to_value(&d).unwrap();
        for field in ["tokens", "entities", "sentences", "noun_chunks"] {
            if actual[field] != c[field] {
                differences.push(serde_json::json!({"case":i,"text":text,"field":field,"actual":actual[field],"expected":c[field]}));
            }
        }
    }
    std::fs::create_dir_all("../../target/reports").unwrap();
    std::fs::write(format!("../../target/reports/{split}-mismatches.json"),serde_json::to_string_pretty(&serde_json::json!({"versions":fixture["versions"],"model":"en_core_web_md 3.8.0","fixture":format!("fixtures/{split}.expected.json"),"tokens":fixture["cases"].as_array().unwrap().iter().map(|c|c["tokens"].as_array().unwrap().len()).sum::<usize>(),"cases":fixture["cases"].as_array().unwrap().len(),"max_tok2vec_absolute_error":max_error,"differences":differences})).unwrap()).unwrap();
    assert!(differences.is_empty(),"{} field mismatches; tok2vec max abs error {max_error}; see target/reports/{split}-mismatches.json",differences.len());
}
#[test]
#[ignore = "requires official export; mandatory CI"]
fn official_regression_parity() {
    check("regression");
}
