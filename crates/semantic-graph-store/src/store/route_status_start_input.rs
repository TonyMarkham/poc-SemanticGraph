use serde_json::Value;

#[derive(Debug, Clone)]
pub struct RouteStatusStartInput<'a> {
    pub workspace_id: i64,
    pub route: &'a str,
    pub scope: &'a str,
    pub scope_key: &'a str,
    pub file_id: Option<i64>,
    pub provider: &'a str,
    pub provider_version: Option<&'a str>,
    pub content_hash: Option<&'a str>,
    pub run_id: i64,
    pub diagnostics_json: Value,
}
