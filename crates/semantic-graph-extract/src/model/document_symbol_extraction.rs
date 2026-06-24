use crate::model::{ExtractedRelation, ExtractedSymbol, GraphLanguage, ProviderId, SourceFile};

use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct DocumentSymbolExtraction {
    pub provider: ProviderId,
    pub language: GraphLanguage,
    pub provider_version: Option<String>,
    pub source_file: SourceFile,
    pub symbols: Vec<ExtractedSymbol>,
    pub relations: Vec<ExtractedRelation>,
    pub raw_metadata: Value,
}
