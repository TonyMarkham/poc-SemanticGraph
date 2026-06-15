use crate::{GraphStoreError, GraphStoreResult, GraphStoreStats};

use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use std::{fs, path::Path};

#[derive(Debug, Clone)]
pub struct GraphStore {
    pool: SqlitePool,
}

impl GraphStore {
    pub async fn connect(path: impl AsRef<Path>) -> GraphStoreResult<Self> {
        let path = path.as_ref();
        Self::create_database_parent(path)?;

        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .map_err(GraphStoreError::database)?;

        Ok(Self { pool })
    }

    fn create_database_parent(path: &Path) -> GraphStoreResult<()> {
        let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        else {
            return Ok(());
        };

        fs::create_dir_all(parent).map_err(|source| {
            GraphStoreError::io(
                "create database parent directory",
                Some(parent.to_path_buf()),
                source,
            )
        })
    }

    pub async fn workspace_id(&self, root_uri: &str) -> GraphStoreResult<Option<i64>> {
        sqlx::query_scalar("SELECT id FROM workspaces WHERE root_uri = ?")
            .bind(root_uri)
            .fetch_optional(&self.pool)
            .await
            .map_err(GraphStoreError::database)
    }

    pub async fn file_id(&self, workspace_id: i64, uri: &str) -> GraphStoreResult<Option<i64>> {
        sqlx::query_scalar("SELECT id FROM files WHERE workspace_id = ? AND uri = ?")
            .bind(workspace_id)
            .bind(uri)
            .fetch_optional(&self.pool)
            .await
            .map_err(GraphStoreError::database)
    }

    pub async fn node_exists(&self, node_id: &str) -> GraphStoreResult<bool> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM nodes WHERE id = ?")
            .bind(node_id)
            .fetch_one(&self.pool)
            .await
            .map_err(GraphStoreError::database)?;

        Ok(count > 0)
    }

    pub async fn stats(&self) -> GraphStoreResult<GraphStoreStats> {
        Ok(GraphStoreStats {
            workspaces: sqlx::query_scalar("SELECT COUNT(*) FROM workspaces")
                .fetch_one(&self.pool)
                .await
                .map_err(GraphStoreError::database)?,
            extraction_runs: sqlx::query_scalar("SELECT COUNT(*) FROM extraction_runs")
                .fetch_one(&self.pool)
                .await
                .map_err(GraphStoreError::database)?,
            files: sqlx::query_scalar("SELECT COUNT(*) FROM files")
                .fetch_one(&self.pool)
                .await
                .map_err(GraphStoreError::database)?,
            nodes: sqlx::query_scalar("SELECT COUNT(*) FROM nodes")
                .fetch_one(&self.pool)
                .await
                .map_err(GraphStoreError::database)?,
            edges: sqlx::query_scalar("SELECT COUNT(*) FROM edges")
                .fetch_one(&self.pool)
                .await
                .map_err(GraphStoreError::database)?,
            occurrences: sqlx::query_scalar("SELECT COUNT(*) FROM occurrences")
                .fetch_one(&self.pool)
                .await
                .map_err(GraphStoreError::database)?,
            edge_evidence: sqlx::query_scalar("SELECT COUNT(*) FROM edge_evidence")
                .fetch_one(&self.pool)
                .await
                .map_err(GraphStoreError::database)?,
        })
    }
}
