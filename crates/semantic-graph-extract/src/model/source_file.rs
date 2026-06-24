use crate::model::SourceLanguage;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFile {
    pub uri: String,
    pub relative_path: String,
    pub language: SourceLanguage,
    pub file_symbol_key: String,
    pub content_hash: Option<String>,
}
