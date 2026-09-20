use crate::{convert, errors, output::Document, Model};
use napi::{bindgen_prelude::Utf16String, Env, Result, Task};
use napi_derive::napi;
use std::sync::Arc;

pub struct LoadTask {
    pub(crate) path: Utf16String,
}

#[napi]
impl Task for LoadTask {
    type Output = errors::Result<Arc<spars::Model>>;
    type JsValue = Model;

    fn compute(&mut self) -> Result<Self::Output> {
        Ok(errors::text(&self.path)
            .and_then(|path| spars::Model::load(path).map(Arc::new).map_err(Into::into)))
    }

    fn resolve(&mut self, env: Env, output: Self::Output) -> Result<Self::JsValue> {
        output
            .map(|inner| Model { inner })
            .map_err(|error| error.into_napi(env))
    }
}

pub struct ProcessTask {
    pub(crate) model: Arc<spars::Model>,
    pub(crate) text: Utf16String,
    pub(crate) stage: spars::Stage,
}

fn process(model: &spars::Model, text: &[u16], stage: spars::Stage) -> errors::Result<Document> {
    let text = errors::text(text)?;
    convert::document(model.process_until(&text, stage)?)
}

#[napi]
impl Task for ProcessTask {
    type Output = errors::Result<Document>;
    type JsValue = Document;

    fn compute(&mut self) -> Result<Self::Output> {
        Ok(process(&self.model, &self.text, self.stage))
    }

    fn resolve(&mut self, env: Env, output: Self::Output) -> Result<Self::JsValue> {
        output.map_err(|error| error.into_napi(env))
    }
}

pub struct BatchTask {
    pub(crate) model: Arc<spars::Model>,
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
            .map(|text| process(&self.model, text, self.stage))
            .collect())
    }

    fn resolve(&mut self, env: Env, output: Self::Output) -> Result<Self::JsValue> {
        output.map_err(|error| error.into_napi(env))
    }
}
