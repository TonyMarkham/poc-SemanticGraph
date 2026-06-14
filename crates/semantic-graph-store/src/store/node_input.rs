use crate::TextRange;

use serde_json::Value;

#[derive(Debug, Clone)]
pub struct NodeInput<'a> {
    pub workspace_id: i64,
    pub language: &'a str,
    pub kind: &'a str,
    pub name: &'a str,
    pub qualified_name: Option<&'a str>,
    pub display_name: Option<&'a str>,
    pub symbol_key: &'a str,
    pub file_id: Option<i64>,
    pub range: Option<TextRange>,
    pub selection_range: Option<TextRange>,
    pub container_node_id: Option<&'a str>,
    pub properties_json: Value,
    pub run_id: Option<i64>,
}
