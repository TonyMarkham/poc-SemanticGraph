use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LaunchConfig {
    pub(crate) binary: PathBuf,
    pub(crate) solution: PathBuf,
    pub(crate) log_level: String,
    pub(crate) features: Vec<String>,
    pub(crate) startup_timeout_ms: u64,
    pub(crate) request_timeout_ms: u64,
}
