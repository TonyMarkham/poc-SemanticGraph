use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NodeSummary {
    pub node_id: String,
    pub kind: String,
    pub name: String,
    pub display_label: String,
    pub qualified_name: Option<String>,
    pub language: String,
    pub source_file_path: Option<String>,
    pub valid_to_run_id: Option<i64>,
}
