use serde_json::Value;

use crate::model::{ExtractedRelation, ExtractedSymbol, ProviderId, SourceFile};

#[derive(Debug, Clone, PartialEq)]
pub struct DocumentSymbolExtraction {
    pub provider: ProviderId,
    pub provider_version: Option<String>,
    pub source_file: SourceFile,
    pub symbols: Vec<ExtractedSymbol>,
    pub relations: Vec<ExtractedRelation>,
    pub raw_metadata: Value,
}
