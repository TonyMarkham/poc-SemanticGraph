use crate::ActiveFileSymbol;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveFileSymbols {
    pub uri: String,
    pub relative_path: String,
    pub language: String,
    pub content_hash: Option<String>,
    pub properties_json: String,
    pub symbols: Vec<ActiveFileSymbol>,
}
