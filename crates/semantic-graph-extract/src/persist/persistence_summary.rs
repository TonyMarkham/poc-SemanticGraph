#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistenceSummary {
    pub workspace_id: i64,
    pub run_id: i64,
    pub files: usize,
    pub nodes: usize,
    pub edges: usize,
    pub occurrences: usize,
    pub evidence: usize,
}
