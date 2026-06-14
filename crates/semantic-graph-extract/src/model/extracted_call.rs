use crate::model::{CallOccurrence, ProviderId};

use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct ExtractedCall {
    pub provider: ProviderId,
    pub caller_symbol_key: String,
    pub callee_symbol_key: String,
    pub context: String,
    pub confidence: String,
    pub confidence_score: f64,
    pub occurrences: Vec<CallOccurrence>,
    pub raw_json: Value,
}
