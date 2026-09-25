use serde_json::{json, Value};
use spars::Model;
use std::{fs, path::PathBuf};

struct Export(PathBuf);
impl Export {
    fn from_manifest(manifest: &Value) -> Self {
        let path = PathBuf::from(format!(
            "../../target/model-loading-{}-{}",
            std::process::id(),
            ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        fs::write(
            path.join("manifest.json"),
            serde_json::to_vec(manifest).unwrap(),
        )
        .unwrap();
        fs::hard_link(
            "../../assets/en_core_web_md-3.8.0/weights.safetensors",
            path.join("weights.safetensors"),
        )
        .unwrap();
        Self(path)
    }
}
static ID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
#[derive(serde::Deserialize)]
struct ReorderedFixture {
    reordered_model: String,
    reordered_pipeline: Vec<String>,
    reordered: Vec<ReorderedCase>,
}
#[derive(serde::Deserialize)]
struct ReorderedCase {
    text: String,
    tokens: Vec<spars::Token>,
    entities: Vec<spars::Span>,
    sentences: Vec<spars::Span>,
    noun_chunks: Vec<spars::Span>,
}
impl Drop for Export {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn manifest() -> Value {
    let mut value: Value = serde_json::from_slice(
        &fs::read("../../assets/en_core_web_md-3.8.0/manifest.json").unwrap(),
    )
    .unwrap();
    value["format_version"] = json!(2);
    value["model"] = json!("independent_model_identity");
    value["model_version"] = json!("1.0.0");
    value["capabilities"] = json!({
        "language": "en", "tok2vec": "spacy.Tok2Vec.v2",
        "embed": "spacy.MultiHashEmbed.v2", "encode": "spacy.MaxoutWindowEncoder.v2",
        "tagger": "spacy.Tagger.v2", "transition": "spacy.TransitionBasedParser.v2",
        "lemmatizer": "en-rule-v1"
    });
    value
}

#[test]
#[ignore = "requires official export; mandatory acceptance test"]
fn model_identity_is_independent_of_native_capabilities() {
    let directory = Export::from_manifest(&manifest());
    let model = Model::load(&directory.0).unwrap();
    let baseline = Model::load("../../assets/en_core_web_md-3.8.0").unwrap();
    for text in [
        "",
        "Alice visited London.",
        "Wait—didn't we meet yesterday?",
    ] {
        assert_eq!(
            model.process(text).unwrap(),
            baseline.process(text).unwrap()
        );
    }
}

#[test]
#[ignore = "requires official export; mandatory acceptance test"]
fn component_order_comes_from_configuration() {
    let mut config = manifest();
    config["pipeline"] = json!([
        "ner",
        "tok2vec",
        "tagger",
        "parser",
        "attribute_ruler",
        "lemmatizer"
    ]);
    let directory = Export::from_manifest(&config);
    let model = Model::load(&directory.0).unwrap();
    let baseline = Model::load("../../assets/en_core_web_md-3.8.0").unwrap();
    assert_eq!(
        model.process("Alice visited London.").unwrap(),
        baseline.process("Alice visited London.").unwrap()
    );
    let doc = model
        .process_until("Alice visited London.", spars::Stage::Ner)
        .unwrap();
    assert!(doc.entities().is_some());
    assert!(doc.tokens()[0].tag.is_none());
    let reference: ReorderedFixture = serde_json::from_slice(
        &fs::read("../../fixtures/model-capabilities-v1.expected.json").unwrap(),
    )
    .unwrap();
    assert_eq!(reference.reordered_model, "en_core_web_md 3.8.0");
    assert_eq!(config["pipeline"], json!(reference.reordered_pipeline));
    assert_eq!(reference.reordered.len(), 3);
    for case in reference.reordered {
        let doc = model.process(&case.text).unwrap();
        assert_eq!(doc.tokens(), case.tokens, "{}", case.text);
        assert_eq!(doc.entities(), Some(case.entities.as_slice()));
        assert_eq!(doc.sentences(), Some(case.sentences.as_slice()));
        assert_eq!(doc.noun_chunks(), Some(case.noun_chunks.as_slice()));
    }
}

#[test]
#[ignore = "requires official export; mandatory acceptance test"]
fn unsupported_capabilities_and_dependencies_fail_before_inference() {
    for omit in [true, false] {
        let mut config = manifest();
        if omit {
            config.as_object_mut().unwrap().remove("capabilities");
        } else {
            config["capabilities"] = Value::Null;
        }
        let directory = Export::from_manifest(&config);
        assert!(Model::load(&directory.0)
            .err()
            .unwrap()
            .to_string()
            .contains("v2 requires explicit runtime capabilities"));
    }
    for (field, value) in [("language", "de"), ("encode", "unknown.encoder.v1")] {
        let mut config = manifest();
        config["capabilities"][field] = json!(value);
        let directory = Export::from_manifest(&config);
        assert!(Model::load(&directory.0).is_err());
    }
    for pipeline in [
        json!(["tagger", "tok2vec"]),
        json!(["tok2vec", "tok2vec"]),
        json!(["lemmatizer"]),
    ] {
        let mut config = manifest();
        config["pipeline"] = pipeline;
        let directory = Export::from_manifest(&config);
        assert!(Model::load(&directory.0).is_err());
    }
    let mut legacy = manifest();
    legacy["format_version"] = json!(1);
    let directory = Export::from_manifest(&legacy);
    assert!(Model::load(&directory.0).is_err());
}

#[test]
#[ignore = "requires official export; mandatory acceptance test"]
fn absent_component_keeps_annotations_unavailable_and_stage_request_fails() {
    let mut config = manifest();
    config["pipeline"] = json!(["tok2vec", "tagger"]);
    let directory = Export::from_manifest(&config);
    let model = Model::load(&directory.0).unwrap();
    let doc = model.process("Alice visited London.").unwrap();
    assert!(doc.tokens()[0].tag.is_some());
    assert!(doc.tokens()[0].head.is_none());
    assert!(doc.entities().is_none());
    assert!(model
        .process_until("hello", spars::Stage::Ner)
        .err()
        .unwrap()
        .to_string()
        .contains("pipeline does not contain Ner"));
}
