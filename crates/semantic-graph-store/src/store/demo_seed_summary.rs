#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DemoSeedSummary {
    pub workspace_id: i64,
    pub run_id: i64,
    pub file_id: i64,
    pub caller_node_id: String,
    pub callee_node_id: String,
    pub edge_id: String,
}
