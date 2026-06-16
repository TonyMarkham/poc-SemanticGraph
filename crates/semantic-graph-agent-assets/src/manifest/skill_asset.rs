use serde::Deserialize;
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillAsset {
    pub name: String,
    pub description: String,
    pub output_path: PathBuf,
    #[serde(default)]
    pub fragments: Vec<PathBuf>,
}
