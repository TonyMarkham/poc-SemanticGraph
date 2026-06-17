use crate::fts::FtsSkipReason;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FtsExtractionSummary {
    pub workspace_id: i64,
    pub run_id: i64,
    pub scanned_files: usize,
    pub indexed_files: usize,
    pub skipped_files: usize,
    pub skipped_directories: usize,
    pub skipped_by_config: usize,
    pub skipped_by_no_rust: usize,
    pub skipped_by_no_csharp: usize,
    pub skipped_by_no_submodules: usize,
    pub skipped_binary_or_unreadable: usize,
    pub stale_fts_documents_closed: u64,
}

impl FtsExtractionSummary {
    pub(crate) fn count_indexed_file(&mut self) {
        self.indexed_files += 1;
    }

    pub(crate) fn count_runtime_skip(&mut self, reason: FtsSkipReason) {
        self.skipped_files += 1;
        if reason == FtsSkipReason::BinaryOrUnreadable {
            self.skipped_binary_or_unreadable += 1;
        }
    }
}
