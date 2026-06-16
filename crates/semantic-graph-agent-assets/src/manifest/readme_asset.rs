use serde::Deserialize;
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadmeAsset {
    pub output_path: PathBuf,
    #[serde(default)]
    pub fragments: Vec<PathBuf>,
}
