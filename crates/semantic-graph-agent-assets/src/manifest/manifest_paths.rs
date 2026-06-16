use serde::Deserialize;
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManifestPaths {
    pub expected_root: PathBuf,
    pub fragment_root: PathBuf,
}
