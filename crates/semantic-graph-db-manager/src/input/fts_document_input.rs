use serde_json::Value;

#[derive(Debug, Clone)]
pub struct FtsDocumentInput<'a> {
    pub workspace_id: i64,
    pub file_id: i64,
    pub path: &'a str,
    pub language: &'a str,
    pub content_hash: &'a str,
    pub byte_len: i64,
    pub run_id: i64,
    pub content: &'a str,
    pub properties_json: Value,
}
