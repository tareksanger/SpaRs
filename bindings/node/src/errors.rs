use napi::{Env, JsError};

pub type Result<T> = std::result::Result<T, BindingError>;

#[derive(Clone)]
pub struct BindingError {
    code: Code,
    message: String,
}

#[derive(Clone)]
enum Code {
    Io,
    InvalidModel,
    Unsupported,
    InvalidText,
    Bounds,
    Inference,
}

impl AsRef<str> for Code {
    fn as_ref(&self) -> &str {
        match self {
            Self::Io => "SPARS_IO",
            Self::InvalidModel => "SPARS_INVALID_MODEL",
            Self::Unsupported => "SPARS_UNSUPPORTED",
            Self::InvalidText => "SPARS_INVALID_TEXT",
            Self::Bounds => "SPARS_BOUNDS",
            Self::Inference => "SPARS_INFERENCE",
        }
    }
}

impl BindingError {
    pub fn into_napi(self, env: Env) -> napi::Error {
        // Create the JS error only on the JS thread. Worker tasks carry Rust data.
        napi::Error::from(
            JsError::from(napi::Error::new(self.code, self.message)).into_unknown(env),
        )
    }
}

impl From<spars::Error> for BindingError {
    fn from(error: spars::Error) -> Self {
        let code = match &error {
            spars::Error::Io(_) => Code::Io,
            spars::Error::Json(_) | spars::Error::Model(_) => Code::InvalidModel,
            spars::Error::Unsupported(_) => Code::Unsupported,
            spars::Error::Bounds => Code::Bounds,
            _ => Code::Inference,
        };
        Self {
            code,
            message: error.to_string(),
        }
    }
}

pub fn text(units: &[u16]) -> Result<String> {
    String::from_utf16(units).map_err(|_| BindingError {
        code: Code::InvalidText,
        message: "Text contains an unpaired UTF-16 surrogate".into(),
    })
}

pub fn number(value: usize) -> Result<u32> {
    u32::try_from(value).map_err(|_| BindingError {
        code: Code::Bounds,
        message: "Document offset or token index exceeds the Node binding's u32 range".into(),
    })
}

impl From<spars_model::Error> for BindingError {
    fn from(error: spars_model::Error) -> Self {
        if let spars_model::Error::Model(error) = error {
            return error.into();
        }
        let code = match &error {
            spars_model::Error::Io(_) => Code::Io,
            _ => Code::InvalidModel,
        };
        Self {
            code,
            message: error.to_string(),
        }
    }
}
