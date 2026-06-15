use serde_json::Value;

#[derive(Debug, Clone)]
pub struct FileInput<'a> {
    pub workspace_id: i64,
    pub uri: &'a str,
    pub path: &'a str,
    pub language: &'a str,
    pub content_hash: Option<&'a str>,
    pub last_seen_run_id: Option<i64>,
    pub properties_json: Value,
}
