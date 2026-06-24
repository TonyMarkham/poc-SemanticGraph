use crate::model::{DocumentSymbolExtraction, GraphLanguage, ProviderId};

use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct DocumentSymbolBatchExtraction {
    pub provider: ProviderId,
    pub language: GraphLanguage,
    pub provider_version: Option<String>,
    pub extractions: Vec<DocumentSymbolExtraction>,
    pub raw_metadata: Value,
}
