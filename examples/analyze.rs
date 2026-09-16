use spars::{Model, Result};
fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .unwrap_or_else(|| "assets/en_core_web_md-3.8.0".into());
    let text = args
        .next()
        .unwrap_or_else(|| "Apple is looking at buying a U.K. startup for $1 billion.".into());
    let m = Model::load(path)?;
    println!("{}", serde_json::to_string_pretty(&m.process(&text)?)?);
    Ok(())
}
