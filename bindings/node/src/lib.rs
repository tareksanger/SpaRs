//! Native Node-API boundary for SpaRs.
mod convert;
mod errors;
mod output;
mod tasks;

use napi::bindgen_prelude::{AsyncTask, Either, Float32Array, Null, Utf16String};
use napi::{Env, Result};
use napi_derive::napi;
use std::sync::Arc;
use tasks::{BatchTask, LoadTask, ProcessTask};

/// An ordered prefix of the pipeline. Later annotations remain null.
#[napi(string_enum)]
pub enum Stage {
    Tokenizer,
    Tagger,
    Parser,
    AttributeRuler,
    Lemmatizer,
    Ner,
}

impl From<Stage> for spars::Stage {
    fn from(value: Stage) -> Self {
        match value {
            Stage::Tokenizer => Self::Tokenizer,
            Stage::Tagger => Self::Tagger,
            Stage::Parser => Self::Parser,
            Stage::AttributeRuler => Self::AttributeRuler,
            Stage::Lemmatizer => Self::Lemmatizer,
            Stage::Ner => Self::Ner,
        }
    }
}

/// A reusable, immutable native model. Create it with loadModel.
#[napi]
pub struct Model {
    inner: Arc<spars::Model>,
}

#[napi]
impl Model {
    /// Run native inference on Node's worker pool. Defaults to the full pipeline.
    #[napi(strict)]
    pub fn process(&self, text: Utf16String, stage: Option<Stage>) -> AsyncTask<ProcessTask> {
        AsyncTask::new(ProcessTask {
            model: Arc::clone(&self.inner),
            text,
            stage: stage.unwrap_or(Stage::Ner).into(),
        })
    }

    /// Process an ordered batch sequentially in one worker job, retaining input order.
    #[napi(strict)]
    pub fn process_batch(
        &self,
        texts: Vec<Utf16String>,
        stage: Option<Stage>,
    ) -> AsyncTask<BatchTask> {
        AsyncTask::new(BatchTask {
            model: Arc::clone(&self.inner),
            texts,
            stage: stage.unwrap_or(Stage::Ner).into(),
        })
    }

    /// Return an independent copy of a static vector, or null when no row exists.
    #[napi(strict)]
    pub fn vector(&self, env: Env, word: Utf16String) -> Result<Either<Float32Array, Null>> {
        let word = errors::text(&word).map_err(|error| error.into_napi(env))?;
        Ok(match self.inner.vector(&word) {
            Some(vector) => Either::A(Float32Array::new(vector.to_vec())),
            None => Either::B(Null),
        })
    }
}

/// Load and validate an installed model on Node's worker pool. No network access.
#[napi(strict)]
pub fn load_model(path: Utf16String) -> AsyncTask<LoadTask> {
    AsyncTask::new(LoadTask { path })
}
