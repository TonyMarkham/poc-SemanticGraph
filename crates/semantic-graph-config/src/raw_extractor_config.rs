use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct RawExtractorConfig {
    pub(crate) mode: Option<String>,
    pub(crate) jobs: Option<usize>,
    pub(crate) reference_jobs: Option<usize>,
    pub(crate) call_jobs: Option<usize>,
    pub(crate) analysis_workers: Option<usize>,
    pub(crate) reference_analysis_workers: Option<usize>,
    pub(crate) call_analysis_workers: Option<usize>,
}
