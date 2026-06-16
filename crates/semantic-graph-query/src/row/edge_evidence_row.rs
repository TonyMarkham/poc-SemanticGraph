use crate::{QueryResult, model::EdgeEvidence, sqlite::parse_optional_json_value};

use sqlx::FromRow;

#[derive(Debug, FromRow)]
pub(crate) struct EdgeEvidenceRow {
    pub(crate) evidence_id: i64,
    pub(crate) run_id: i64,
    pub(crate) provider: String,
    pub(crate) lsp_method: Option<String>,
    pub(crate) source_file_path: Option<String>,
    pub(crate) start_line: Option<i64>,
    pub(crate) start_col: Option<i64>,
    pub(crate) end_line: Option<i64>,
    pub(crate) end_col: Option<i64>,
    pub(crate) raw_json: Option<String>,
}

impl EdgeEvidenceRow {
    pub(crate) fn into_model(self) -> QueryResult<EdgeEvidence> {
        Ok(EdgeEvidence {
            evidence_id: self.evidence_id,
            run_id: self.run_id,
            provider: self.provider,
            lsp_method: self.lsp_method,
            source_file_path: self.source_file_path,
            start_line: self.start_line,
            start_col: self.start_col,
            end_line: self.end_line,
            end_col: self.end_col,
            raw_json: parse_optional_json_value(self.raw_json)?,
        })
    }
}
