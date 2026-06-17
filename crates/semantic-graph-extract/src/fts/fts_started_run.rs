#[derive(Debug, Clone)]
pub(crate) struct FtsStartedRun {
    pub(crate) workspace_root_uri: String,
    pub(crate) workspace_id: i64,
    pub(crate) run_id: i64,
    pub(crate) analysis_workers: usize,
}
