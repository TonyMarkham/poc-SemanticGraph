use serde_json::Value;

#[derive(Debug, Clone)]
pub struct RouteStatusFailInput<'a> {
    pub workspace_id: i64,
    pub route: &'a str,
    pub scope: &'a str,
    pub scope_key: &'a str,
    pub provider: &'a str,
    pub run_id: i64,
    pub diagnostics_json: Value,
}
