use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct RawSoulScanConfig {
    pub(crate) excluded_dirs: Option<Vec<String>>,
    pub(crate) excluded_dir_suffixes: Option<Vec<String>>,
    pub(crate) excluded_bin_except_under: Option<Vec<String>>,
}
