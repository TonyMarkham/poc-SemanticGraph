#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StaleFileSummary {
    pub file_id: Option<i64>,
    pub stale_nodes_closed: u64,
    pub stale_edges_closed: u64,
}
