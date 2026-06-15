use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliExtractorPlanOptions {
    pub explicit_config_path: Option<PathBuf>,
    pub workspace_root: PathBuf,
    pub serial: bool,
    pub jobs: Option<usize>,
    pub reference_jobs: Option<usize>,
    pub call_jobs: Option<usize>,
    pub analysis_workers: Option<usize>,
    pub reference_analysis_workers: Option<usize>,
    pub call_analysis_workers: Option<usize>,
}
