#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistenceSummary {
    pub workspace_id: i64,
    pub run_id: i64,
    pub files: usize,
    pub nodes: usize,
    pub edges: usize,
    pub reference_edges: usize,
    pub call_edges: usize,
    pub occurrences: usize,
    pub reference_occurrences: usize,
    pub call_occurrences: usize,
    pub evidence: usize,
    pub routes_complete: usize,
    pub stale_nodes_closed: usize,
    pub stale_edges_closed: usize,
}
