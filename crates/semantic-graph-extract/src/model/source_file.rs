use crate::model::GraphLanguage;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFile {
    pub uri: String,
    pub relative_path: String,
    pub language: GraphLanguage,
    pub file_symbol_key: String,
    pub content_hash: Option<String>,
}
