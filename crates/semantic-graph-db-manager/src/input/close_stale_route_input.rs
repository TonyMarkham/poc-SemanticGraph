#[derive(Debug, Clone)]
pub struct CloseStaleRouteInput<'a> {
    pub workspace_id: i64,
    pub run_id: i64,
    pub route: &'a str,
    pub scope: &'a str,
    pub scope_key: &'a str,
    pub provider: &'a str,
}
