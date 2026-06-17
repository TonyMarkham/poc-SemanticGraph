#[derive(Debug, Clone, Copy)]
pub(crate) struct FtsFileWorkerConfig {
    pub(crate) workspace_id: i64,
    pub(crate) run_id: i64,
    pub(crate) max_indexed_file_bytes: u64,
    pub(crate) route: &'static str,
}
