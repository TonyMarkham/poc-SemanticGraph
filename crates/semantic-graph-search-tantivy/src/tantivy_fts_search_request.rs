#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TantivyFtsSearchRequest {
    pub query: String,
    pub limit: usize,
    pub offset: usize,
    pub language: Option<String>,
    pub path_prefix: Option<String>,
    pub case_sensitive: bool,
}
