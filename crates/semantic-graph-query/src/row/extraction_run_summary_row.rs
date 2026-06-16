use crate::{QueryResult, model::ExtractionRunSummary, sqlite::parse_json_value};

use sqlx::FromRow;

#[derive(Debug, FromRow)]
pub(crate) struct ExtractionRunSummaryRow {
    pub(crate) run_id: i64,
    pub(crate) workspace_id: i64,
    pub(crate) root_uri: String,
    pub(crate) provider: String,
    pub(crate) provider_version: Option<String>,
    pub(crate) git_commit: Option<String>,
    pub(crate) started_at: String,
    pub(crate) finished_at: Option<String>,
    pub(crate) status: String,
    pub(crate) properties_json: String,
}

impl ExtractionRunSummaryRow {
    pub(crate) fn into_model(self) -> QueryResult<ExtractionRunSummary> {
        Ok(ExtractionRunSummary {
            run_id: self.run_id,
            workspace_id: self.workspace_id,
            root_uri: self.root_uri,
            provider: self.provider,
            provider_version: self.provider_version,
            git_commit: self.git_commit,
            started_at: self.started_at,
            finished_at: self.finished_at,
            status: self.status,
            properties_json: parse_json_value(&self.properties_json)?,
        })
    }
}
