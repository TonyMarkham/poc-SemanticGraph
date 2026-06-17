use serde_json::Value;

#[derive(Debug, Clone)]
pub struct FtsWriteBatchDocumentInput {
    pub workspace_id: i64,
    pub uri: String,
    pub path: String,
    pub language: String,
    pub content_hash: String,
    pub byte_len: i64,
    pub run_id: i64,
    pub content: String,
    pub properties_json: Value,
}
