use crate::model::EdgeEndpoint;

use sqlx::FromRow;

#[derive(Debug, FromRow)]
pub(crate) struct EdgeEndpointRow {
    pub(crate) node_id: String,
    pub(crate) kind: String,
    pub(crate) name: String,
    pub(crate) display_label: String,
    pub(crate) qualified_name: Option<String>,
    pub(crate) language: String,
    pub(crate) source_file_path: Option<String>,
    pub(crate) valid_to_run_id: Option<i64>,
}

impl EdgeEndpointRow {
    pub(crate) fn into_model(self) -> EdgeEndpoint {
        EdgeEndpoint {
            node_id: self.node_id,
            kind: self.kind,
            name: self.name,
            display_label: self.display_label,
            qualified_name: self.qualified_name,
            language: self.language,
            source_file_path: self.source_file_path,
            valid_to_run_id: self.valid_to_run_id,
        }
    }
}
