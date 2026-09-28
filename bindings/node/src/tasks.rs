use crate::{convert, errors, output::Document, Model};
use napi::{bindgen_prelude::Utf16String, Env, Result, Task};
use napi_derive::napi;
use std::sync::Arc;

pub struct LoadTask {
    pub(crate) source: errors::Result<crate::models::LoadSource>,
}

#[napi]
impl Task for LoadTask {
    type Output = errors::Result<Arc<spars::Model>>;
    type JsValue = Model;

    fn compute(&mut self) -> Result<Self::Output> {
        Ok(self
            .source
            .as_ref()
            .map_err(Clone::clone)
            .and_then(|source| source.load().map(Arc::new)))
    }

    fn resolve(&mut self, env: Env, output: Self::Output) -> Result<Self::JsValue> {
        output
            .map(|inner| Model {
                batch_size: inner.default_batch_size(),
                inner,
                max_length: 1_000_000,
            })
            .map_err(|error| error.into_napi(env))
    }
}

pub struct ProcessTask {
    pub(crate) model: Arc<spars::Model>,
    pub(crate) max_length: u32,
    pub(crate) text: Utf16String,
    pub(crate) stage: spars::Stage,
}

fn process(
    model: &spars::Model,
    text: &[u16],
    stage: spars::Stage,
    max_length: u32,
) -> errors::Result<spars::Doc> {
    let text = errors::text(text)?;
    if text.len() > max_length as usize {
        let length = text.chars().count();
        if length > max_length as usize {
            return Err(errors::text_too_long(length, max_length));
        }
    }
    Ok(model.process_until(&text, stage)?)
}

#[napi]
impl Task for ProcessTask {
    type Output = errors::Result<Document>;
    type JsValue = Document;

    fn compute(&mut self) -> Result<Self::Output> {
        Ok(process(
            &self.model,
            &self.text,
            self.stage,
            self.max_length,
        )
        .and_then(|doc| convert::document(&doc)))
    }

    fn resolve(&mut self, env: Env, output: Self::Output) -> Result<Self::JsValue> {
        output.map_err(|error| error.into_napi(env))
    }
}

pub struct BatchTask {
    pub(crate) model: Arc<spars::Model>,
    pub(crate) max_length: u32,
    pub(crate) texts: Vec<Utf16String>,
    pub(crate) stage: spars::Stage,
}

#[napi]
impl Task for BatchTask {
    type Output = errors::Result<Vec<Document>>;
    type JsValue = Vec<Document>;

    fn compute(&mut self) -> Result<Self::Output> {
        Ok(self
            .texts
            .iter()
            .map(|text| {
                process(&self.model, text, self.stage, self.max_length)
                    .and_then(|doc| convert::document(&doc))
            })
            .collect())
    }

    fn resolve(&mut self, env: Env, output: Self::Output) -> Result<Self::JsValue> {
        output.map_err(|error| error.into_napi(env))
    }
}

pub struct ProcessDocumentTask {
    pub(crate) model: Arc<spars::Model>,
    pub(crate) max_length: u32,
    pub(crate) text: Utf16String,
    pub(crate) stage: spars::Stage,
}

#[napi]
impl Task for ProcessDocumentTask {
    type Output = errors::Result<spars::Doc>;
    type JsValue = crate::document::NativeDocument;

    fn compute(&mut self) -> Result<Self::Output> {
        Ok(process(&self.model, &self.text, self.stage, self.max_length))
    }

    fn resolve(&mut self, env: Env, output: Self::Output) -> Result<Self::JsValue> {
        output
            .map(|doc| crate::document::NativeDocument { inner: Arc::new(doc) })
            .map_err(|error| error.into_napi(env))
    }
}
