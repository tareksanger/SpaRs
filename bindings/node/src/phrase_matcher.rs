//! Reusable exact phrase matching over immutable native documents.
use crate::{document::NativeDocument, errors};
use napi::{
    bindgen_prelude::{AsyncTask, ClassInstance, Utf16String},
    Env, Result, Task,
};
use napi_derive::napi;
use std::sync::Arc;

#[napi(object)]
pub struct PhraseMatch {
    pub rule: String,
    pub start: u32,
    pub end: u32,
}

#[napi]
pub struct PhraseMatcher {
    inner: Arc<spars::PhraseMatcher>,
}

#[napi]
impl PhraseMatcher {
    /// Match exact token text using ORTH (default) or TEXT, pinned Unicode LOWER, NORM, or the
    /// LEMMA, POS, TAG, DEP and MORPH annotations.
    #[napi(constructor, strict)]
    pub fn new(env: Env, attribute: Option<Utf16String>) -> Result<Self> {
        let attribute = attribute
            .as_ref()
            .map(|value| errors::text(value))
            .transpose()
            .map_err(|error| error.into_napi(env))?;
        let attribute = match attribute.as_deref().unwrap_or("ORTH") {
            "ORTH" => spars::PhraseAttribute::Orth,
            "TEXT" => spars::PhraseAttribute::Text,
            "LOWER" => spars::PhraseAttribute::Lower,
            "NORM" => spars::PhraseAttribute::Norm,
            "LEMMA" => spars::PhraseAttribute::Lemma,
            "POS" => spars::PhraseAttribute::Pos,
            "TAG" => spars::PhraseAttribute::Tag,
            "DEP" => spars::PhraseAttribute::Dep,
            "MORPH" => spars::PhraseAttribute::Morph,
            _ => {
                return Err(errors::BindingError::from(spars::Error::Unsupported(
                    "PhraseMatcher supports only ORTH, TEXT, LOWER, NORM, LEMMA, POS, TAG, DEP, and MORPH".into(),
                ))
                .into_napi(env))
            }
        };
        Ok(Self {
            inner: Arc::new(spars::PhraseMatcher::with_attribute(attribute)),
        })
    }

    #[napi(getter)]
    pub fn attribute(&self) -> &'static str {
        match self.inner.attribute() {
            spars::PhraseAttribute::Orth => "ORTH",
            spars::PhraseAttribute::Text => "TEXT",
            spars::PhraseAttribute::Lower => "LOWER",
            spars::PhraseAttribute::Norm => "NORM",
            spars::PhraseAttribute::Lemma => "LEMMA",
            spars::PhraseAttribute::Pos => "POS",
            spars::PhraseAttribute::Tag => "TAG",
            spars::PhraseAttribute::Dep => "DEP",
            spars::PhraseAttribute::Morph => "MORPH",
        }
    }

    #[napi(getter)]
    pub fn size(&self, env: Env) -> Result<u32> {
        errors::number(self.inner.len()).map_err(|error| error.into_napi(env))
    }

    #[napi(strict)]
    pub fn contains(&self, env: Env, rule: Utf16String) -> Result<bool> {
        let rule = errors::text(&rule).map_err(|error| error.into_napi(env))?;
        Ok(self.inner.contains(&rule))
    }

    /// Return owned unique nonempty patterns of compared token values in registration order.
    #[napi(strict)]
    pub fn get(&self, env: Env, rule: Utf16String) -> Result<Option<Vec<Vec<String>>>> {
        let rule = errors::text(&rule).map_err(|error| error.into_napi(env))?;
        Ok(self.inner.get(&rule).map(|patterns| {
            patterns
                .iter()
                .map(|pattern| pattern.tokens().to_vec())
                .collect()
        }))
    }

    /// Copy the compared pattern token values into the trie. Pending searches prevent mutation.
    #[napi(strict)]
    pub fn add(
        &mut self,
        env: Env,
        rule: Utf16String,
        patterns: Vec<ClassInstance<'_, NativeDocument>>,
    ) -> Result<()> {
        let rule = errors::text(&rule).map_err(|error| error.into_napi(env))?;
        let matcher = Arc::get_mut(&mut self.inner).ok_or_else(|| errors::busy().into_napi(env))?;
        let docs: Vec<&spars::Doc> = patterns
            .iter()
            .map(|pattern| pattern.inner.as_ref())
            .collect();
        matcher
            .add(rule, &docs)
            .map_err(|error| errors::registration(error).into_napi(env))
    }

    #[napi(strict)]
    pub fn remove(&mut self, env: Env, rule: Utf16String) -> Result<()> {
        let rule = errors::text(&rule).map_err(|error| error.into_napi(env))?;
        let matcher = Arc::get_mut(&mut self.inner).ok_or_else(|| errors::busy().into_napi(env))?;
        matcher
            .remove(&rule)
            .map_err(|error| errors::BindingError::from(error).into_napi(env))
    }

    /// Return all overlapping matches in native reference order on Node's worker pool.
    #[napi(strict, ts_return_type = "Promise<Array<PhraseMatch>>")]
    pub fn find_matches(&self, doc: &NativeDocument) -> AsyncTask<PhraseMatchTask> {
        AsyncTask::new(PhraseMatchTask {
            matcher: Arc::clone(&self.inner),
            doc: Arc::clone(&doc.inner),
        })
    }
}

pub struct PhraseMatchTask {
    matcher: Arc<spars::PhraseMatcher>,
    doc: Arc<spars::Doc>,
}

#[napi]
impl Task for PhraseMatchTask {
    type Output = errors::Result<Vec<PhraseMatch>>;
    type JsValue = Vec<PhraseMatch>;

    fn compute(&mut self) -> Result<Self::Output> {
        Ok(self
            .matcher
            .find_matches(&self.doc)
            .map_err(Into::into)
            .and_then(|matches| {
                matches
                    .into_iter()
                    .map(|matched| {
                        Ok(PhraseMatch {
                            rule: matched.rule.0,
                            start: errors::number(matched.start.0)?,
                            end: errors::number(matched.end.0)?,
                        })
                    })
                    .collect()
            }))
    }

    fn resolve(&mut self, env: Env, output: Self::Output) -> Result<Self::JsValue> {
        output.map_err(|error| error.into_napi(env))
    }
}
