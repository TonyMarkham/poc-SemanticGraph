use crate::args::ResolvedServerConfig;

use semantic_graph_query::{FtsQueryService, GraphQueryService};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ServerState {
    database_path: PathBuf,
    fts_database_path: Option<PathBuf>,
    fts_index_path: Option<PathBuf>,
    query_service: GraphQueryService,
    fts_query_service: Option<FtsQueryService>,
}

impl ServerState {
    pub fn from_resolved_config(config: ResolvedServerConfig) -> Self {
        let database_path = config.database_path().clone();
        let query_service = GraphQueryService::with_query_service_config(
            database_path.clone(),
            config.query_service_config().clone(),
        );
        let fts_database_path = config.fts_database_path().cloned();
        let fts_index_path = config.fts_index_path().cloned();
        let fts_query_service = fts_database_path.clone().zip(fts_index_path.clone()).map(
            |(fts_database_path, fts_index_path)| {
                FtsQueryService::with_query_service_config(
                    fts_database_path,
                    fts_index_path,
                    config.query_service_config().clone(),
                )
            },
        );

        Self {
            database_path,
            fts_database_path,
            fts_index_path,
            query_service,
            fts_query_service,
        }
    }

    pub fn database_path(&self) -> &Path {
        &self.database_path
    }

    pub fn query_service(&self) -> &GraphQueryService {
        &self.query_service
    }

    pub fn fts_database_path(&self) -> Option<&Path> {
        self.fts_database_path.as_deref()
    }

    pub fn fts_index_path(&self) -> Option<&Path> {
        self.fts_index_path.as_deref()
    }

    pub fn fts_query_service(&self) -> Option<&FtsQueryService> {
        self.fts_query_service.as_ref()
    }
}
