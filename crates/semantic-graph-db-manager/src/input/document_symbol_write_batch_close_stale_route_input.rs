#[derive(Debug, Clone)]
pub struct DocumentSymbolWriteBatchCloseStaleRouteInput {
    pub workspace_id: i64,
    pub run_id: i64,
    pub route: String,
    pub scope: String,
    pub scope_key: String,
    pub provider: String,
}
