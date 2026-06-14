#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphStoreStats {
    pub workspaces: i64,
    pub extraction_runs: i64,
    pub files: i64,
    pub nodes: i64,
    pub edges: i64,
    pub occurrences: i64,
    pub edge_evidence: i64,
}
