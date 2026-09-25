use crate::{invalid, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::{fmt, num::NonZeroU32, str::FromStr};
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Digest(String);
impl TryFrom<String> for Digest {
    type Error = crate::Error;
    fn try_from(value: String) -> Result<Self> {
        if value.len() != 64
            || !value
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(invalid(
                "SHA-256 must contain 64 lowercase hexadecimal digits",
            ));
        }
        Ok(Self(value))
    }
}
impl From<Digest> for String {
    fn from(v: Digest) -> Self {
        v.0
    }
}
impl fmt::Display for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl Digest {
    pub fn of(bytes: &[u8]) -> Self {
        Self(format!("{:x}", Sha256::digest(bytes)))
    }
    pub(crate) fn from_hash(hash: Sha256) -> Self {
        Self(format!("{:x}", hash.finalize()))
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ModelName(std::borrow::Cow<'static, str>);
impl ModelName {
    /// Legacy spelling retained for callers; model names are validated data.
    #[allow(non_upper_case_globals)]
    pub const EnCoreWebMd: Self = Self(std::borrow::Cow::Borrowed("en_core_web_md"));
}
impl FromStr for ModelName {
    type Err = crate::Error;
    fn from_str(value: &str) -> Result<Self> {
        if value.is_empty()
            || value.len() > 128
            || !value.as_bytes()[0].is_ascii_lowercase()
            || !value
                .bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_')
        {
            return Err(invalid("model name must start with a lowercase ASCII letter and contain only lowercase letters, digits and underscores (maximum 128 bytes)"));
        }
        Ok(Self(std::borrow::Cow::Owned(value.to_owned())))
    }
}
impl TryFrom<String> for ModelName {
    type Error = crate::Error;
    fn try_from(value: String) -> Result<Self> {
        value.parse()
    }
}
impl From<ModelName> for String {
    fn from(value: ModelName) -> Self {
        value.0.into_owned()
    }
}
impl fmt::Display for ModelName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}
impl FromStr for Version {
    type Err = crate::Error;
    fn from_str(s: &str) -> Result<Self> {
        let parts = s
            .split('.')
            .map(|p| {
                if p.is_empty()
                    || (p.len() > 1 && p.starts_with('0'))
                    || !p.bytes().all(|b| b.is_ascii_digit())
                {
                    return Err(invalid("version must use three canonical numeric parts"));
                }
                p.parse::<u32>()
                    .map_err(|_| invalid("version number overflow"))
            })
            .collect::<Result<Vec<_>>>()?;
        match parts.as_slice() {
            [major, minor, patch] => Ok(Self {
                major: *major,
                minor: *minor,
                patch: *patch,
            }),
            _ => Err(invalid("version must use major.minor.patch")),
        }
    }
}
impl TryFrom<String> for Version {
    type Error = crate::Error;
    fn try_from(v: String) -> Result<Self> {
        v.parse()
    }
}
impl From<Version> for String {
    fn from(v: Version) -> Self {
        format!("{}.{}.{}", v.major, v.minor, v.patch)
    }
}
impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Identity {
    pub model: ModelName,
    pub model_version: Version,
    pub wheel_sha256: Digest,
    pub recipe_revision: NonZeroU32,
    pub resource_revision: NonZeroU32,
    pub format_version: NonZeroU32,
}
impl Identity {
    pub fn directory_name(&self) -> String {
        format!(
            "{}-{}-f{}-r{}-l{}-{}",
            self.model,
            self.model_version,
            self.format_version,
            self.recipe_revision,
            self.resource_revision,
            self.wheel_sha256
        )
    }
}
#[cfg(test)]
mod tests;
