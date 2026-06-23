use crate::model::{NodeSummary, SoulLinkedSource};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SoulSearchResult {
    pub workspace_id: i64,
    pub root_uri: String,
    pub soul_id: String,
    pub document: Option<NodeSummary>,
    pub source_annotations: Vec<SoulLinkedSource>,
    pub markdown_references: Vec<SoulLinkedSource>,
    pub has_document: bool,
    pub source_annotation_count: usize,
    pub linked_source_annotation_count: usize,
    pub markdown_reference_count: usize,
}
