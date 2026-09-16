//! Native Rust inference for an explicitly exported en_core_web_md 3.8.0 model.
//!
//! Model acquisition is separate from Cargo builds and inference. Python is only
//! needed to produce official model exports and reference fixtures.
//!
//! ```no_run
//! use spars::{Model, TokenIndex};
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let model = Model::load("assets/en_core_web_md-3.8.0")?;
//! let doc = model.process("Alice works in London.")?;
//! assert_eq!(doc.token(TokenIndex(0))?.text(), "Alice");
//! assert!(doc.entities().is_some());
//! # Ok(())
//! # }
//! ```
//!
//! Native snapshots distinguish unavailable annotations from computed empty ones:
//! ```
//! use spars::Doc;
//! let json = r#"{"format_version":1,"document":{"text":"","tokens":[],"entities":null,"sentences":null,"noun_chunks":null}}"#;
//! let doc = Doc::from_json(json).unwrap();
//! assert!(doc.entities().is_none());
//! assert_eq!(Doc::from_json(&doc.to_json().unwrap()).unwrap(), doc);
//! ```
//!
//! Compatibility is limited to the pinned English inference configuration.
//! Training, additional architectures/languages and the wider spaCy API remain
//! unimplemented. See the repository compatibility matrix and parity reports.
mod document;
mod hash;
mod lexical;
pub use lexical::Lexeme;
mod attributes;
mod chunks;
mod lemmatizer;
mod model;
mod ner;
mod neural;
mod parser;
mod pipeline;
mod tokenizer;
mod validation;
pub use document::*;
pub use model::Model;
pub use pipeline::Stage;
use thiserror::Error;
#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid model: {0}")]
    Model(String),
    #[error("unsupported configuration: {0}")]
    Unsupported(String),
    #[error("invalid span or offset")]
    Bounds,
    #[error("regular expression: {0}")]
    Regex(Box<fancy_regex::Error>),
}
pub type Result<T> = std::result::Result<T, Error>;
#[cfg(test)]
mod tests;

impl From<fancy_regex::Error> for Error {
    fn from(e: fancy_regex::Error) -> Self {
        Self::Regex(Box::new(e))
    }
}
