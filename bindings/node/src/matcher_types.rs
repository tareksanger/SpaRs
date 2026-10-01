//! Concrete shared matcher inputs; external strings are validated before compilation.
use crate::errors;
use napi::bindgen_prelude::{Either, Utf16String};
use napi_derive::napi;

#[napi(object)]
pub struct TokenPredicate {
    pub kind: Utf16String,
    /// A string for `equals`; a boolean for `flag`.
    pub value: Option<Either<Utf16String, bool>>,
    pub values: Option<Vec<Utf16String>>,
}

#[napi(object)]
pub struct TokenConstraint {
    pub attribute: Utf16String,
    pub predicate: TokenPredicate,
}

pub(crate) fn invalid(message: &str) -> errors::BindingError {
    spars::Error::Pattern(message.into()).into()
}

impl TokenConstraint {
    pub(crate) fn into_native(self) -> errors::Result<spars::TokenConstraint> {
        let attribute = match errors::text(&self.attribute)?.as_str() {
            "text" => spars::TokenAttribute::Text,
            "lower" => spars::TokenAttribute::Lower,
            "norm" => spars::TokenAttribute::Norm,
            "lemma" => spars::TokenAttribute::Lemma,
            "pos" => spars::TokenAttribute::Pos,
            "tag" => spars::TokenAttribute::Tag,
            "dep" => spars::TokenAttribute::Dep,
            "morphology" => spars::TokenAttribute::Morphology,
            "is_alpha" => spars::TokenAttribute::IsAlpha,
            "is_digit" => spars::TokenAttribute::IsDigit,
            "is_space" => spars::TokenAttribute::IsSpace,
            "is_punct" => spars::TokenAttribute::IsPunct,
            "like_num" => spars::TokenAttribute::LikeNum,
            _ => return Err(invalid("unknown token attribute")),
        };
        let kind = errors::text(&self.predicate.kind)?;
        let predicate = if kind == "equals" || kind == "flag" {
            if self.predicate.values.is_some() {
                return Err(invalid("equals and flag require value and forbid values"));
            }
            match (kind.as_str(), self.predicate.value) {
                ("equals", Some(Either::A(value))) => spars::Predicate::Equals {
                    value: errors::text(&value)?,
                },
                ("flag", Some(Either::B(value))) => spars::Predicate::Flag { value },
                ("equals", _) => return Err(invalid("equals requires a string value")),
                _ => return Err(invalid("flag requires a boolean value")),
            }
        } else {
            if self.predicate.value.is_some() {
                return Err(invalid("set predicates require values and forbid value"));
            }
            let values = self
                .predicate
                .values
                .ok_or_else(|| invalid("set predicates require values"))?
                .iter()
                .map(|value| errors::text(value))
                .collect::<errors::Result<Vec<_>>>()?;
            match kind.as_str() {
                "in" => spars::Predicate::In { values },
                "not_in" => spars::Predicate::NotIn { values },
                "morph_superset" => spars::Predicate::MorphSuperset { values },
                "morph_intersects" => spars::Predicate::MorphIntersects { values },
                _ => return Err(invalid("unknown token predicate kind")),
            }
        };
        Ok(spars::TokenConstraint {
            attribute,
            predicate,
        })
    }

    pub(crate) fn from_native(value: &spars::TokenConstraint) -> Self {
        let attribute = match value.attribute {
            spars::TokenAttribute::Text => "text",
            spars::TokenAttribute::Lower => "lower",
            spars::TokenAttribute::Norm => "norm",
            spars::TokenAttribute::Lemma => "lemma",
            spars::TokenAttribute::Pos => "pos",
            spars::TokenAttribute::Tag => "tag",
            spars::TokenAttribute::Dep => "dep",
            spars::TokenAttribute::Morphology => "morphology",
            spars::TokenAttribute::IsAlpha => "is_alpha",
            spars::TokenAttribute::IsDigit => "is_digit",
            spars::TokenAttribute::IsSpace => "is_space",
            spars::TokenAttribute::IsPunct => "is_punct",
            spars::TokenAttribute::LikeNum => "like_num",
        };
        let (kind, value, values) = match &value.predicate {
            spars::Predicate::Equals { value } => {
                ("equals", Some(Either::A(value.clone().into())), None)
            }
            spars::Predicate::Flag { value } => ("flag", Some(Either::B(*value)), None),
            spars::Predicate::In { values } => ("in", None, Some(values)),
            spars::Predicate::NotIn { values } => ("not_in", None, Some(values)),
            spars::Predicate::MorphSuperset { values } => ("morph_superset", None, Some(values)),
            spars::Predicate::MorphIntersects { values } => {
                ("morph_intersects", None, Some(values))
            }
        };
        Self {
            attribute: attribute.to_owned().into(),
            predicate: TokenPredicate {
                kind: kind.to_owned().into(),
                value,
                values: values.map(|values| values.iter().cloned().map(Into::into).collect()),
            },
        }
    }
}
