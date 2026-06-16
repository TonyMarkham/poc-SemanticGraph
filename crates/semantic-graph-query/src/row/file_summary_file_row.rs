use crate::{QueryResult, model::FileSummaryFile, sqlite::parse_json_value};

use sqlx::FromRow;

#[derive(Debug, FromRow)]
pub(crate) struct FileSummaryFileRow {
    pub(crate) workspace_id: i64,
    pub(crate) root_uri: String,
    pub(crate) file_id: i64,
    pub(crate) uri: String,
    pub(crate) path: String,
    pub(crate) language: String,
    pub(crate) content_hash: Option<String>,
    pub(crate) last_seen_run_id: Option<i64>,
    pub(crate) properties_json: String,
}

impl FileSummaryFileRow {
    pub(crate) fn into_model(self) -> QueryResult<(i64, String, FileSummaryFile)> {
        let workspace_id = self.workspace_id;
        let root_uri = self.root_uri;
        let file = FileSummaryFile {
            file_id: self.file_id,
            uri: self.uri,
            path: self.path,
            language: self.language,
            content_hash: self.content_hash,
            last_seen_run_id: self.last_seen_run_id,
            properties_json: parse_json_value(&self.properties_json)?,
        };

        Ok((workspace_id, root_uri, file))
    }
}
