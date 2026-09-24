use spars::{Doc, Model, TokenIndex};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = Model::load(std::env::args().nth(1).expect("model directory"))?;
    let text = "Alice works at Microsoft in New York.";
    let doc = model.process(text)?;
    assert_eq!(doc.text(), text);
    assert_eq!(doc.tokens().len(), 8);
    assert!(doc.tokens().iter().all(|t| t.tag.is_some()
        && t.pos.is_some()
        && t.morphology.is_some()
        && t.lemma.is_some()
        && t.head.is_some()
        && t.dep.is_some()
        && t.sentence_start.is_some()
        && t.entity_iob.is_some()));
    assert!(doc.entities().is_some());
    assert!(doc.sentences().is_some());
    assert!(doc.noun_chunks().is_some());
    let vector = model.document_vector(&doc);
    assert!(!vector.is_empty());
    assert!(vector.iter().all(|value| value.is_finite()));
    assert_eq!(
        model.token_vector(doc.token(TokenIndex(3))?).unwrap().len(),
        vector.len()
    );
    assert_eq!(Doc::from_json(&doc.to_json()?)?, doc);
    for (i, t) in doc.tokens().iter().enumerate() {
        println!(
            "{}\t{}\t{}\t{}\t{}",
            doc.token_text(TokenIndex(i))?,
            t.tag.as_deref().unwrap(),
            t.lemma.as_deref().unwrap(),
            t.head.unwrap().0,
            t.dep.as_deref().unwrap()
        );
    }
    Ok(())
}
