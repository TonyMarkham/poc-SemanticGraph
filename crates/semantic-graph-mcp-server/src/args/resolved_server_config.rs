use semantic_graph_config::{QueryServiceConfig, ResolvedDatabasePathSource};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ResolvedServerConfig {
    database_path: PathBuf,
    database_path_source: ResolvedDatabasePathSource,
    query_service_config: QueryServiceConfig,
}

impl ResolvedServerConfig {
    pub fn new(
        database_path: PathBuf,
        database_path_source: ResolvedDatabasePathSource,
        query_service_config: QueryServiceConfig,
    ) -> Self {
        Self {
            database_path,
            database_path_source,
            query_service_config,
        }
    }

    pub fn database_path(&self) -> &PathBuf {
        &self.database_path
    }

    pub fn database_path_source(&self) -> ResolvedDatabasePathSource {
        self.database_path_source
    }

    pub fn query_service_config(&self) -> &QueryServiceConfig {
        &self.query_service_config
    }
}
