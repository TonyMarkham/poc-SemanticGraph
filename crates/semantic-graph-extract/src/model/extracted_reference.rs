use crate::model::{ProviderId, ReferenceOccurrence};

use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct ExtractedReference {
    pub provider: ProviderId,
    pub source_symbol_key: String,
    pub target_symbol_key: String,
    pub source_resolution: String,
    pub confidence: String,
    pub confidence_score: f64,
    pub occurrences: Vec<ReferenceOccurrence>,
    pub raw_json: Value,
}
