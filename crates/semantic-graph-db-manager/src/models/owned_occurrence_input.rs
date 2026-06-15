use crate::{OccurrenceInput, TextRange};

use serde_json::Value;

#[derive(Debug, Clone)]
pub(crate) struct OwnedOccurrenceInput {
    pub(crate) node_id: String,
    pub(crate) run_id: i64,
    pub(crate) file_id: i64,
    pub(crate) role: String,
    pub(crate) range: TextRange,
    pub(crate) enclosing_node_id: Option<String>,
    pub(crate) raw_json: Option<Value>,
}

impl From<OccurrenceInput<'_>> for OwnedOccurrenceInput {
    fn from(input: OccurrenceInput<'_>) -> Self {
        Self {
            node_id: input.node_id.to_string(),
            run_id: input.run_id,
            file_id: input.file_id,
            role: input.role.to_string(),
            range: input.range,
            enclosing_node_id: input.enclosing_node_id.map(str::to_string),
            raw_json: input.raw_json,
        }
    }
}
