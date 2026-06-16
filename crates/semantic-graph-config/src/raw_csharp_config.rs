use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct RawCSharpConfig {
    pub(crate) binary: Option<String>,
    pub(crate) solution: Option<String>,
    pub(crate) log_level: Option<String>,
    pub(crate) features: Option<Vec<String>>,
    pub(crate) analysis_workers: Option<usize>,
    pub(crate) startup_timeout_ms: Option<u64>,
    pub(crate) request_timeout_ms: Option<u64>,
}
