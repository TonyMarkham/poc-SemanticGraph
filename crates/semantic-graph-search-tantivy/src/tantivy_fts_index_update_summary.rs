#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TantivyFtsIndexUpdateSummary {
    pub indexed_documents: usize,
    pub deleted_uris: usize,
    pub committed: bool,
    pub indexing_workers: usize,
    pub memory_budget_bytes: usize,
}
