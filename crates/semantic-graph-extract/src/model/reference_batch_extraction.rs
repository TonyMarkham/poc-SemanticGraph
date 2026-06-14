use crate::model::{
    DocumentSymbolBatchExtraction, ExtractedReference, ProviderId, ReferenceRouteSummary,
};

use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct ReferenceBatchExtraction {
    pub provider: ProviderId,
    pub provider_version: Option<String>,
    pub workspace_fingerprint: String,
    pub document_symbols: DocumentSymbolBatchExtraction,
    pub references: Vec<ExtractedReference>,
    pub summary: ReferenceRouteSummary,
    pub raw_metadata: Value,
}
