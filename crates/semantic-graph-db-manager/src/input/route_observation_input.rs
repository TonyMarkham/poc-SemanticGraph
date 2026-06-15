use serde_json::Value;

#[derive(Debug, Clone)]
pub struct RouteObservationInput<'a> {
    pub workspace_id: i64,
    pub run_id: i64,
    pub route: &'a str,
    pub scope: &'a str,
    pub scope_key: &'a str,
    pub provider: &'a str,
    pub entity_kind: &'a str,
    pub entity_id: &'a str,
    pub source_file_id: Option<i64>,
    pub properties_json: Value,
}
