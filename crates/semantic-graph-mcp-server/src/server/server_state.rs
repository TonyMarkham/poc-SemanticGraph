use crate::args::ResolvedServerConfig;

use semantic_graph_query::GraphQueryService;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ServerState {
    database_path: PathBuf,
    query_service: GraphQueryService,
}

impl ServerState {
    pub fn from_resolved_config(config: ResolvedServerConfig) -> Self {
        let database_path = config.database_path().clone();
        let query_service = GraphQueryService::with_query_service_config(
            database_path.clone(),
            config.query_service_config().clone(),
        );

        Self {
            database_path,
            query_service,
        }
    }

    pub fn database_path(&self) -> &Path {
        &self.database_path
    }

    pub fn query_service(&self) -> &GraphQueryService {
        &self.query_service
    }
}
