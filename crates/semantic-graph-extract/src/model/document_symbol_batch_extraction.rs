use serde_json::Value;

use crate::model::{DocumentSymbolExtraction, ProviderId};

#[derive(Debug, Clone, PartialEq)]
pub struct DocumentSymbolBatchExtraction {
    pub provider: ProviderId,
    pub provider_version: Option<String>,
    pub extractions: Vec<DocumentSymbolExtraction>,
    pub raw_metadata: Value,
}
