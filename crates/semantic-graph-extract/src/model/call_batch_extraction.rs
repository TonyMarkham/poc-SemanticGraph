use crate::model::{CallRouteSummary, DocumentSymbolBatchExtraction, ExtractedCall, ProviderId};

use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct CallBatchExtraction {
    pub provider: ProviderId,
    pub provider_version: Option<String>,
    pub workspace_fingerprint: String,
    pub document_symbols: DocumentSymbolBatchExtraction,
    pub calls: Vec<ExtractedCall>,
    pub summary: CallRouteSummary,
    pub raw_metadata: Value,
}
