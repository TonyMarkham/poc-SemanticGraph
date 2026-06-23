use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub(crate) struct RawSoulPluginConfig {
    pub(crate) language: Option<String>,
    pub(crate) path: Option<PathBuf>,
}
