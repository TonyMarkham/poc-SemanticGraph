use crate::{NodeInput, TextRange};

use serde_json::Value;

#[derive(Debug, Clone)]
pub(crate) struct OwnedNodeInput {
    pub(crate) workspace_id: i64,
    pub(crate) language: String,
    pub(crate) kind: String,
    pub(crate) name: String,
    pub(crate) qualified_name: Option<String>,
    pub(crate) display_name: Option<String>,
    pub(crate) symbol_key: String,
    pub(crate) file_id: Option<i64>,
    pub(crate) range: Option<TextRange>,
    pub(crate) selection_range: Option<TextRange>,
    pub(crate) container_node_id: Option<String>,
    pub(crate) properties_json: Value,
    pub(crate) run_id: Option<i64>,
}

impl From<NodeInput<'_>> for OwnedNodeInput {
    fn from(input: NodeInput<'_>) -> Self {
        Self {
            workspace_id: input.workspace_id,
            language: input.language.to_string(),
            kind: input.kind.to_string(),
            name: input.name.to_string(),
            qualified_name: input.qualified_name.map(str::to_string),
            display_name: input.display_name.map(str::to_string),
            symbol_key: input.symbol_key.to_string(),
            file_id: input.file_id,
            range: input.range,
            selection_range: input.selection_range,
            container_node_id: input.container_node_id.map(str::to_string),
            properties_json: input.properties_json,
            run_id: input.run_id,
        }
    }
}
