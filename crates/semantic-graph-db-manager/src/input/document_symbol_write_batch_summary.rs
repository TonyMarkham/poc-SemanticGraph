#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DocumentSymbolWriteBatchSummary {
    pub stale_nodes_closed: u64,
    pub stale_edges_closed: u64,
}
