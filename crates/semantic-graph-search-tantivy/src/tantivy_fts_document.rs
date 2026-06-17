#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TantivyFtsDocument {
    pub uri: String,
    pub path: String,
    pub language: String,
    pub content_hash: String,
    pub content: String,
}
