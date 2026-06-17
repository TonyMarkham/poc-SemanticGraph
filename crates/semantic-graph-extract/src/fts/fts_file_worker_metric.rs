use std::time::Duration;

#[derive(Debug, Clone)]
pub(crate) struct FtsFileWorkerMetric {
    pub(crate) worker_index: usize,
    pub(crate) files: usize,
    pub(crate) files_hashed: usize,
    pub(crate) files_changed: usize,
    pub(crate) files_hash_unchanged: usize,
    pub(crate) skipped_binary_or_unreadable: usize,
    pub(crate) elapsed: Duration,
}
