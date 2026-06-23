use crate::model::{EdgeSummary, NodeSummary};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SoulLinkedSource {
    pub source: NodeSummary,
    pub edge: Option<EdgeSummary>,
    pub source_file_language: String,
    pub start_line: Option<i64>,
    pub start_col: Option<i64>,
    pub end_line: Option<i64>,
    pub end_col: Option<i64>,
}
