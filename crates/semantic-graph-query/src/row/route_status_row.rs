use crate::{QueryResult, model::RouteStatus, sqlite::parse_json_value};

use sqlx::FromRow;

#[derive(Debug, FromRow)]
pub(crate) struct RouteStatusRow {
    pub(crate) route_status_id: i64,
    pub(crate) workspace_id: i64,
    pub(crate) root_uri: String,
    pub(crate) route: String,
    pub(crate) scope: String,
    pub(crate) scope_key: String,
    pub(crate) file_path: Option<String>,
    pub(crate) provider: String,
    pub(crate) provider_version: Option<String>,
    pub(crate) content_hash: Option<String>,
    pub(crate) last_started_run_id: Option<i64>,
    pub(crate) last_complete_run_id: Option<i64>,
    pub(crate) last_status: String,
    pub(crate) diagnostics_json: String,
    pub(crate) updated_at: String,
}

impl RouteStatusRow {
    pub(crate) fn into_model(self) -> QueryResult<RouteStatus> {
        Ok(RouteStatus {
            route_status_id: self.route_status_id,
            workspace_id: self.workspace_id,
            root_uri: self.root_uri,
            route: self.route,
            scope: self.scope,
            scope_key: self.scope_key,
            file_path: self.file_path,
            provider: self.provider,
            provider_version: self.provider_version,
            content_hash: self.content_hash,
            last_started_run_id: self.last_started_run_id,
            last_complete_run_id: self.last_complete_run_id,
            last_status: self.last_status,
            diagnostics_json: parse_json_value(&self.diagnostics_json)?,
            updated_at: self.updated_at,
        })
    }
}
