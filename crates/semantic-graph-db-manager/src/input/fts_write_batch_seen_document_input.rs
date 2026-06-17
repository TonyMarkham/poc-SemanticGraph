#[derive(Debug, Clone)]
pub struct FtsWriteBatchSeenDocumentInput {
    pub workspace_id: i64,
    pub uri: String,
    pub content_hash: String,
    pub run_id: i64,
}
