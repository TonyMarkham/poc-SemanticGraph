#[derive(Debug, Clone, PartialEq)]
pub struct TantivyFtsSearchHit {
    pub uri: String,
    pub path: String,
    pub language: String,
    pub content_hash: String,
    pub score: f32,
}
