use crate::{convert, errors, output::Document};
use napi::{bindgen_prelude::Utf16String, Env, Result};
use napi_derive::napi;
use std::sync::Arc;

/// An immutable native document, independent of its originating model.
#[napi]
pub struct NativeDocument {
    pub(crate) inner: Arc<spars::Doc>,
    utf16_length: u32,
}

impl NativeDocument {
    pub(crate) fn from_doc(doc: spars::Doc) -> errors::Result<Self> {
        let utf16_length = errors::number(doc.text().encode_utf16().count())?;
        Ok(Self {
            inner: Arc::new(doc),
            utf16_length,
        })
    }
}

#[napi]
impl NativeDocument {
    /// Text length in JavaScript UTF-16 units without constructing an output copy.
    #[napi(getter)]
    pub fn utf16_length(&self) -> u32 {
        self.utf16_length
    }

    /// Restore a validated native snapshot. This is not spaCy's JSON format.
    #[napi(factory, strict)]
    pub fn from_snapshot(env: Env, json: Utf16String) -> Result<Self> {
        let json = errors::text(&json).map_err(|error| error.into_napi(env))?;
        spars::Doc::from_json(&json)
            .map_err(errors::BindingError::from)
            .and_then(Self::from_doc)
            .map_err(|error| error.into_napi(env))
    }

    /// Serialize all native annotations and contextual vectors, when present.
    #[napi]
    pub fn to_snapshot(&self, env: Env) -> Result<String> {
        self.inner
            .to_json()
            .map_err(|error| errors::BindingError::from(error).into_napi(env))
    }

    /// Return independent plain output objects with byte, code-point and UTF-16 offsets.
    #[napi]
    pub fn to_object(&self, env: Env) -> Result<Document> {
        convert::document(&self.inner).map_err(|error| error.into_napi(env))
    }
}
