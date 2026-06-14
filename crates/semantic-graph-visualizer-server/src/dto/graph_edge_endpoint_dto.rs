use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GraphEdgeEndpointDto {
    pub node_id: String,
    pub kind: String,
    pub display_label: String,
    pub qualified_name: Option<String>,
    pub language: String,
    pub source_file_path: Option<String>,
}
