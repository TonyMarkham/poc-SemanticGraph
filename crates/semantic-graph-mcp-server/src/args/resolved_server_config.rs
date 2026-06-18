use semantic_graph_config::{QueryServiceConfig, ResolvedDatabasePathSource};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ResolvedServerConfig {
    database_path: PathBuf,
    database_path_source: ResolvedDatabasePathSource,
    fts_database_path: Option<PathBuf>,
    fts_index_path: Option<PathBuf>,
    query_service_config: QueryServiceConfig,
}

impl ResolvedServerConfig {
    pub fn new(
        database_path: PathBuf,
        database_path_source: ResolvedDatabasePathSource,
        fts_database_path: Option<PathBuf>,
        fts_index_path: Option<PathBuf>,
        query_service_config: QueryServiceConfig,
    ) -> Self {
        Self {
            database_path,
            database_path_source,
            fts_database_path,
            fts_index_path,
            query_service_config,
        }
    }

    pub fn database_path(&self) -> &PathBuf {
        &self.database_path
    }

    pub fn database_path_source(&self) -> ResolvedDatabasePathSource {
        self.database_path_source
    }

    pub fn fts_database_path(&self) -> Option<&PathBuf> {
        self.fts_database_path.as_ref()
    }

    pub fn fts_index_path(&self) -> Option<&PathBuf> {
        self.fts_index_path.as_ref()
    }

    pub fn query_service_config(&self) -> &QueryServiceConfig {
        &self.query_service_config
    }
}
