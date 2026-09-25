//! Inspect dependency relationships with an explicitly loaded model.
use spars::{Model, TokenIndex};

fn main() -> spars::Result<()> {
    let model = Model::load("assets/en_core_web_md-3.8.0")?;
    let doc = model.process("Alice works in London.")?;
    let verb = doc.token(TokenIndex(1))?;
    println!("Sentence: {}", verb.sentence()?.text());
    for child in verb.children()? {
        println!("{} -> {}", verb.text(), child.text());
    }
    assert_eq!(verb.subtree()?.count(), doc.tokens().len());
    Ok(())
}
