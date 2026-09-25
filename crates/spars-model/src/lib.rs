//! Explicit native installation of pinned official spaCy packages.
mod archive;
mod catalog;
mod convert;
mod download;
mod identity;
mod install;
mod integrity;
mod lookups;
mod project;
mod recipe;
pub use project::{download_model, DownloadOptions};
mod tensors;
mod unique_json;
pub use identity::{Digest, Identity, ModelName, Version};
pub use install::{install, list, verify, InstalledModel};
pub use recipe::catalog;
use thiserror::Error;
#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("archive: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("tensor: {0}")]
    Tensor(#[from] tensors::TensorError),
    #[error("model validation: {0}")]
    Model(#[from] spars::Error),
    #[error("invalid installation data: {0}")]
    Invalid(String),
    #[error("another installation is using this directory")]
    Busy,
}
pub type Result<T> = std::result::Result<T, Error>;
fn invalid(message: impl Into<String>) -> Error {
    Error::Invalid(message.into())
}
