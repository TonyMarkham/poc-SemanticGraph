use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct RawFtsConfig {
    #[serde(rename = "ignore-directories")]
    pub(crate) ignore_directories: Option<Vec<String>>,
    #[serde(rename = "ignore-files")]
    pub(crate) ignore_files: Option<Vec<String>>,
}
