use crate::{EdgeEvidenceInput, TextRange};

use serde_json::Value;

#[derive(Debug, Clone)]
pub(crate) struct OwnedEdgeEvidenceInput {
    pub(crate) edge_id: String,
    pub(crate) run_id: i64,
    pub(crate) provider: String,
    pub(crate) lsp_method: Option<String>,
    pub(crate) file_id: Option<i64>,
    pub(crate) range: Option<TextRange>,
    pub(crate) raw_json: Option<Value>,
}

impl From<EdgeEvidenceInput<'_>> for OwnedEdgeEvidenceInput {
    fn from(input: EdgeEvidenceInput<'_>) -> Self {
        Self {
            edge_id: input.edge_id.to_string(),
            run_id: input.run_id,
            provider: input.provider.to_string(),
            lsp_method: input.lsp_method.map(str::to_string),
            file_id: input.file_id,
            range: input.range,
            raw_json: input.raw_json,
        }
    }
}
