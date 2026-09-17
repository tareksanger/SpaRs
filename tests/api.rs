use spars::{ByteOffset, CodePointOffset, Doc, Model, Stage, TokenIndex};
#[test]
fn missing_model_is_an_error() {
    assert!(Model::load("this-model-does-not-exist").is_err());
}
#[test]
#[ignore = "requires official export; mandatory CI"]
fn public_api_and_batch_isolation() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<Model>();
    let m = Model::load("assets/en_core_web_md-3.8.0").unwrap();
    let texts = ["😀 Café é.  ", "", "Alice met Bob in London."];
    let individual: Vec<_> = texts.iter().map(|s| m.process(s).unwrap()).collect();
    let batched: Vec<_> = m.pipe(texts).map(Result::unwrap).collect();
    assert_eq!(individual, batched);
    std::thread::scope(|scope| {
        let jobs: Vec<_> = texts
            .iter()
            .map(|s| scope.spawn(|| m.process(s).unwrap()))
            .collect();
        for (job, expected) in jobs.into_iter().zip(&individual) {
            assert_eq!(&job.join().unwrap(), expected);
        }
    });
    for d in &individual {
        let restored = Doc::from_json(&d.to_json().unwrap()).unwrap();
        assert_eq!(&restored, d);
        let mut restored_text = String::new();
        for (i, t) in d.tokens().iter().enumerate() {
            restored_text.push_str(d.token_text(TokenIndex(i)).unwrap());
            if t.whitespace {
                restored_text.push(' ')
            }
            assert_eq!(d.codepoint_offset(t.start).unwrap(), t.idx);
            assert_eq!(d.byte_offset(t.idx).unwrap(), t.start);
        }
        assert_eq!(restored_text, d.text());
    }
    let d = &individual[0];
    assert_eq!(d.byte_offset(CodePointOffset(1)).unwrap(), ByteOffset(4));
    assert!(d.codepoint_offset(ByteOffset(1)).is_err());
    assert!(d.span_text(TokenIndex(2), TokenIndex(1)).is_err());
    assert!(d.token_text(TokenIndex(usize::MAX)).is_err());
    let d = m.process_until("Alice met Bob.", Stage::Tagger).unwrap();
    assert!(d.tokens()[0].tag.is_some());
    assert!(d.tokens()[0].head.is_none());
    assert!(d.entities().is_none());
    assert!(d.noun_chunks().is_none());
    let empty = m.process("").unwrap();
    assert_eq!(empty.entities(), Some([].as_slice()));
    assert_eq!(m.document_vector(&empty), vec![0.; 300]);
    assert!(m.vector("xyzzynonword123").is_none());
}
#[test]
#[ignore = "requires official export; mandatory CI"]
fn malformed_models_are_errors() {
    let root = std::path::Path::new("assets/en_core_web_md-3.8.0");
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("manifest.json")).unwrap()).unwrap();
    let tmp = std::env::temp_dir().join(format!("spacy-native-invalid-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).unwrap();
    let weights = tmp.join("weights.safetensors");
    if weights.exists() {
        std::fs::remove_file(&weights).unwrap()
    }
    std::fs::hard_link(root.join("weights.safetensors"), &weights).unwrap();
    let cases = [
        ("/format_version", serde_json::json!(99)),
        ("/tokenizer/faster_heuristics", serde_json::json!(false)),
        ("/versions/spacy", serde_json::json!("0")),
        ("/pipeline", serde_json::json!(["transformer"])),
        ("/weights_sha256", serde_json::json!("bad")),
        ("/tok2vec/width", serde_json::json!(5)),
        ("/tok2vec/attrs/0", serde_json::json!("UNKNOWN")),
        ("/tok2vec/pad", serde_json::json!(90)),
        ("/parser/lower/W", serde_json::json!("absent")),
        ("/parser/actions/0", serde_json::json!("unknown")),
        ("/tokenizer/prefix", serde_json::json!("[")),
        ("/attribute_rules/0/index", serde_json::json!(999)),
        ("/attribute_rules", serde_json::Value::Null),
        ("/lexical/ranges/alpha", serde_json::json!([[4, 1]])),
    ];
    for (pointer, value) in cases {
        let mut c = manifest.clone();
        *c.pointer_mut(pointer).unwrap() = value;
        std::fs::write(tmp.join("manifest.json"), serde_json::to_vec(&c).unwrap()).unwrap();
        assert!(Model::load(&tmp).is_err(), "accepted {pointer}");
    }
    std::fs::remove_file(weights).unwrap();
    std::fs::write(
        tmp.join("manifest.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    assert!(Model::load(&tmp).is_err());
    std::fs::remove_dir_all(tmp).unwrap();
}
#[test]
#[ignore = "requires official export; mandatory CI"]
fn checked_views_and_similarity() {
    let m = Model::load("assets/en_core_web_md-3.8.0").unwrap();
    let d = m.process("dog cat").unwrap();
    let dog = d.token(TokenIndex(0)).unwrap();
    let cat = d.token(TokenIndex(1)).unwrap();
    assert_eq!(dog.text(), "dog");
    assert_eq!(dog.index(), TokenIndex(0));
    assert!(dog.head().is_some());
    assert_eq!(m.token_vector(dog), m.vector("dog"));
    assert!(
        (m.span_similarity(dog.span(), cat.span())
            - m.similarity(&m.tokenize("dog").unwrap(), &m.tokenize("cat").unwrap()))
        .abs()
            < 1e-6
    );
    assert_eq!(m.span_similarity(dog.span(), dog.span()), 1.);
    let empty = m.lexeme("").unwrap();
    assert_eq!(empty.orth, 0);
    assert!(!empty.is_ascii && !empty.is_punct && !empty.is_currency);
    let spans = d.span(TokenIndex(0), TokenIndex(2)).unwrap();
    assert_eq!(
        spans.tokens().map(|t| t.text()).collect::<Vec<_>>(),
        ["dog", "cat"]
    );
}
