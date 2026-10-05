use crate::{convert, errors, output::Document, views};
use napi::bindgen_prelude::{JsObjectValue, Object, Utf16String};
use napi::{Env, Result, Status};
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

/// A labeled half-open token interval to annotate as an entity.
#[napi(object)]
pub struct EntityInput {
    pub start: f64,
    pub end: f64,
    /// An empty label ignores the interval, as spaCy does.
    pub label: Utf16String,
}

/// An unlabeled half-open token interval.
#[napi(object)]
pub struct TokenRange {
    pub start: f64,
    pub end: f64,
}

/// How tokens outside every given interval are annotated, like spaCy's `Doc.set_ents` default.
#[napi(string_enum = "lowercase")]
pub enum EntityDefault {
    Outside,
    Missing,
    Blocked,
    Unmodified,
}

/// A replacement of entity annotation. Intervals in all lists must not share tokens.
#[napi(object)]
pub struct EntityUpdate {
    pub entities: Option<Vec<EntityInput>>,
    /// Tokens that can never be part of an entity.
    pub blocked: Option<Vec<TokenRange>>,
    /// Tokens whose entity annotation is unknown.
    pub missing: Option<Vec<TokenRange>>,
    /// Tokens outside any entity.
    pub outside: Option<Vec<TokenRange>>,
    /// Defaults to `outside`.
    pub default: Option<EntityDefault>,
}

/// napi objects ignore undeclared properties; reject them so a misspelled field such as `ents`
/// or an unsupported one such as `kbId` cannot silently change the update.
fn known_fields(object: &Object, allowed: &[&str]) -> Result<()> {
    let names = object.get_property_names()?;
    for i in 0..names.get_array_length()? {
        let name: String = names.get_element(i)?;
        if !allowed.contains(&name.as_str()) {
            return Err(napi::Error::new(
                Status::InvalidArg,
                format!("unknown entity update field `{name}`"),
            ));
        }
    }
    Ok(())
}

impl EntityUpdate {
    fn from_object(object: &Object) -> Result<Self> {
        known_fields(
            object,
            &["entities", "blocked", "missing", "outside", "default"],
        )?;
        for (name, allowed) in [
            ("entities", &["start", "end", "label"][..]),
            ("blocked", &["start", "end"]),
            ("missing", &["start", "end"]),
            ("outside", &["start", "end"]),
        ] {
            for item in object
                .get_named_property::<Option<Vec<Object>>>(name)?
                .unwrap_or_default()
            {
                known_fields(&item, allowed)?;
            }
        }
        Ok(Self {
            entities: object.get_named_property("entities")?,
            blocked: object.get_named_property("blocked")?,
            missing: object.get_named_property("missing")?,
            outside: object.get_named_property("outside")?,
            default: object.get_named_property("default")?,
        })
    }

    fn native(self, env: Env) -> Result<spars::EntityUpdate> {
        let ranges = |ranges: Option<Vec<TokenRange>>| -> Result<Vec<spars::TokenRange>> {
            ranges
                .unwrap_or_default()
                .into_iter()
                .map(|range| {
                    Ok(spars::TokenRange {
                        start: views::index(range.start, env)?,
                        end: views::index(range.end, env)?,
                    })
                })
                .collect()
        };
        let entities = self
            .entities
            .unwrap_or_default()
            .into_iter()
            .map(|entity| {
                Ok(spars::Span {
                    start: views::index(entity.start, env)?,
                    end: views::index(entity.end, env)?,
                    label: errors::text(&entity.label).map_err(|error| error.into_napi(env))?,
                })
            })
            .collect::<Result<_>>()?;
        Ok(spars::EntityUpdate {
            entities,
            blocked: ranges(self.blocked)?,
            missing: ranges(self.missing)?,
            outside: ranges(self.outside)?,
            default: match self.default.unwrap_or(EntityDefault::Outside) {
                EntityDefault::Outside => spars::EntityDefault::Outside,
                EntityDefault::Missing => spars::EntityDefault::Missing,
                EntityDefault::Blocked => spars::EntityDefault::Blocked,
                EntityDefault::Unmodified => spars::EntityDefault::Unmodified,
            },
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

    /// Return a new document with replaced entity annotation, like spaCy's `Doc.set_ents`.
    /// This document and its token and span views are unchanged. Invalid intervals throw
    /// `SPARS_BOUNDS`, a label with an unpaired surrogate `SPARS_INVALID_TEXT`, tags from an edited
    /// snapshot that spaCy could not read `SPARS_UNSUPPORTED`, and undeclared fields `InvalidArg`.
    #[napi(strict, ts_args_type = "update: EntityUpdate")]
    pub fn with_entities(&self, env: Env, update: Object) -> Result<NativeDocument> {
        let update = EntityUpdate::from_object(&update)?.native(env)?;
        let mut doc = spars::Doc::clone(&self.inner);
        doc.set_entities(&update)
            .map_err(|error| errors::BindingError::from(error).into_napi(env))?;
        Ok(Self {
            inner: std::sync::Arc::new(doc),
            utf16_length: self.utf16_length,
        })
    }

    /// Return independent plain output objects with byte, code-point and UTF-16 offsets.
    #[napi]
    pub fn to_object(&self, env: Env) -> Result<Document> {
        convert::document(&self.inner).map_err(|error| error.into_napi(env))
    }
}
