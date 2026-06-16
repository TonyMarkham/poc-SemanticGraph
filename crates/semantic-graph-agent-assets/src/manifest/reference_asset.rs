use serde::Deserialize;
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceAsset {
    pub name: String,
    pub title: String,
    pub output_path: PathBuf,
    #[serde(default)]
    pub fragments: Vec<PathBuf>,
}
