use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct RawFtsConfig {
    pub(crate) db_path: Option<std::path::PathBuf>,
    pub(crate) analysis_workers: Option<usize>,
    pub(crate) max_indexed_file_bytes: Option<u64>,
    #[serde(rename = "ignore-directories")]
    pub(crate) ignore_directories: Option<Vec<String>>,
    #[serde(rename = "ignore-files")]
    pub(crate) ignore_files: Option<Vec<String>>,
}
