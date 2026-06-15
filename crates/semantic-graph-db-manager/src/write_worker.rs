use crate::{
    DbManagerError, DbManagerResult, DbWriteProgressKind, DemoSeedSummary, EdgeEvidenceInput,
    EdgeInput, FileInput, NodeInput, OccurrenceInput, TextRange, WriteProgress, WriteSummary,
    commands::Commands,
    edge_id,
    models::{
        OwnedCloseStaleRouteInput, OwnedEdgeEvidenceInput, OwnedEdgeInput, OwnedFileInput,
        OwnedNodeInput, OwnedOccurrenceInput, OwnedRouteObservationInput,
        OwnedRouteStatusCompleteInput, OwnedRouteStatusFailInput, OwnedRouteStatusStartInput,
    },
    node_id,
};

use serde_json::json;
use sqlx::{Executor, SqlitePool};
use tokio::sync::{broadcast, mpsc, oneshot};

macro_rules! run_write_command {
    ($worker:ident, $operation:expr) => {{
        match $worker.begin_write().await {
            Ok(()) => {
                let result = $operation.await;
                $worker.finish_write(result).await
            }
            Err(error) => Err(error),
        }
    }};
}

pub(crate) struct WriteWorker {
    pool: SqlitePool,
    receiver: mpsc::Receiver<Commands>,
    progress: broadcast::Sender<WriteProgress>,
    summary: WriteSummary,
}

impl WriteWorker {
    pub(crate) fn new(
        pool: SqlitePool,
        receiver: mpsc::Receiver<Commands>,
        progress: broadcast::Sender<WriteProgress>,
    ) -> Self {
        Self {
            pool,
            receiver,
            progress,
            summary: WriteSummary::default(),
        }
    }

    pub(crate) async fn run(mut self) {
        let mut shutdown_response = None;
        self.emit(DbWriteProgressKind::ManagerStarted);
        while let Some(command) = self.receiver.recv().await {
            self.summary.commands_queued += 1;
            self.emit(DbWriteProgressKind::CommandQueued);
            if let Commands::Shutdown { response } = command {
                shutdown_response = Some(response);
                break;
            }
            self.handle_command(command).await;
        }

        self.pool.close().await;
        self.emit(DbWriteProgressKind::ManagerShutdown);
        if let Some(response) = shutdown_response {
            let _send_result = response.send(Ok(self.summary));
        }
    }

    fn emit(&self, kind: DbWriteProgressKind) {
        let _send_result = self.progress.send(WriteProgress::new(kind, self.summary));
    }

    async fn handle_command(&mut self, command: Commands) {
        let _command_name = command.name();
        let is_write = command.is_write();
        if is_write {
            self.emit(DbWriteProgressKind::CommandWriting);
        }

        match command {
            Commands::Migrate { response } => {
                let result = self.migrate().await;
                self.send_write_response(response, result).await;
            }
            Commands::CreateWorkspace {
                root_uri,
                kind,
                response,
            } => {
                let result = run_write_command!(self, self.create_workspace(&root_uri, &kind));
                self.send_write_response(response, result).await;
            }
            Commands::WorkspaceId { root_uri, response } => {
                let result = self.workspace_id(&root_uri).await;
                let _send_result = response.send(result);
            }
            Commands::FileId {
                workspace_id,
                uri,
                response,
            } => {
                let result = self.file_id(workspace_id, &uri).await;
                let _send_result = response.send(result);
            }
            Commands::NodeExists { node_id, response } => {
                let result = self.node_exists(&node_id).await;
                let _send_result = response.send(result);
            }
            Commands::StartRun {
                workspace_id,
                provider,
                provider_version,
                git_commit,
                response,
            } => {
                let result = run_write_command!(
                    self,
                    self.start_run(
                        workspace_id,
                        &provider,
                        provider_version.as_deref(),
                        git_commit.as_deref(),
                    )
                );
                self.send_write_response(response, result).await;
            }
            Commands::FinishRun {
                run_id,
                status,
                response,
            } => {
                let result = run_write_command!(self, self.finish_run(run_id, &status));
                self.send_write_response(response, result).await;
            }
            Commands::UpsertFile { input, response } => {
                let result = run_write_command!(self, self.upsert_file(input));
                self.send_write_response(response, result).await;
            }
            Commands::UpsertNode { input, response } => {
                let result = run_write_command!(self, self.upsert_node(input));
                self.send_write_response(response, result).await;
            }
            Commands::UpsertEdge { input, response } => {
                let result = run_write_command!(self, self.upsert_edge(input));
                self.send_write_response(response, result).await;
            }
            Commands::InsertOccurrence { input, response } => {
                let result = run_write_command!(self, self.insert_occurrence(input));
                self.send_write_response(response, result).await;
            }
            Commands::InsertEdgeEvidence { input, response } => {
                let result = run_write_command!(self, self.insert_edge_evidence(input));
                self.send_write_response(response, result).await;
            }
            Commands::StartRouteStatus { input, response } => {
                let result = run_write_command!(self, self.start_route_status(input));
                self.send_write_response(response, result).await;
            }
            Commands::CompleteRouteStatus { input, response } => {
                let result = run_write_command!(self, self.complete_route_status(input));
                self.send_write_response(response, result).await;
            }
            Commands::FailRouteStatus { input, response } => {
                let result = run_write_command!(self, self.fail_route_status(input));
                self.send_write_response(response, result).await;
            }
            Commands::RecordRouteObservation { input, response } => {
                let result = run_write_command!(self, self.record_route_observation(input));
                self.send_write_response(response, result).await;
            }
            Commands::CloseStaleNodesForRoute { input, response } => {
                let result = run_write_command!(self, self.close_stale_nodes_for_route(input));
                self.send_write_response(response, result).await;
            }
            Commands::CloseStaleEdgesForRoute { input, response } => {
                let result = run_write_command!(self, self.close_stale_edges_for_route(input));
                self.send_write_response(response, result).await;
            }
            Commands::DemoSeed { root_uri, response } => {
                let result = run_write_command!(self, self.demo_seed(&root_uri));
                self.send_write_response(response, result).await;
            }
            Commands::Shutdown { .. } => {
                unreachable!("shutdown is handled by WriteWorker::run");
            }
        }
    }

    async fn send_write_response<T>(
        &mut self,
        response: oneshot::Sender<DbManagerResult<T>>,
        result: DbManagerResult<T>,
    ) {
        match &result {
            Ok(_) => self.emit(DbWriteProgressKind::CommandCommitted),
            Err(_) => self.emit(DbWriteProgressKind::CommandFailed),
        }
        let _send_result = response.send(result);
    }

    async fn begin_write(&self) -> DbManagerResult<()> {
        self.pool
            .execute("BEGIN IMMEDIATE")
            .await
            .map_err(DbManagerError::database)?;
        Ok(())
    }

    async fn finish_write<T>(&mut self, result: DbManagerResult<T>) -> DbManagerResult<T> {
        match result {
            Ok(value) => {
                self.pool
                    .execute("COMMIT")
                    .await
                    .map_err(DbManagerError::database)?;
                self.summary.commands_written += 1;
                self.summary.commits += 1;
                Ok(value)
            }
            Err(error) => {
                let _rollback_result = self.pool.execute("ROLLBACK").await;
                self.summary.rollbacks += 1;
                Err(error)
            }
        }
    }

    async fn migrate(&mut self) -> DbManagerResult<()> {
        sqlx::migrate!("../semantic-graph-store/migrations")
            .run(&self.pool)
            .await
            .map_err(DbManagerError::migration)?;
        self.pool
            .execute("PRAGMA foreign_keys = ON")
            .await
            .map_err(DbManagerError::database)?;
        self.summary.commands_written += 1;
        self.summary.commits += 1;
        Ok(())
    }

    async fn create_workspace(&self, root_uri: &str, kind: &str) -> DbManagerResult<i64> {
        sqlx::query(
            r#"
            INSERT INTO workspaces (root_uri, kind)
            VALUES (?, ?)
            ON CONFLICT(root_uri) DO UPDATE SET kind = excluded.kind
            "#,
        )
        .bind(root_uri)
        .bind(kind)
        .execute(&self.pool)
        .await
        .map_err(DbManagerError::database)?;

        let id = sqlx::query_scalar("SELECT id FROM workspaces WHERE root_uri = ?")
            .bind(root_uri)
            .fetch_one(&self.pool)
            .await
            .map_err(DbManagerError::database)?;

        Ok(id)
    }

    async fn workspace_id(&self, root_uri: &str) -> DbManagerResult<Option<i64>> {
        sqlx::query_scalar("SELECT id FROM workspaces WHERE root_uri = ?")
            .bind(root_uri)
            .fetch_optional(&self.pool)
            .await
            .map_err(DbManagerError::database)
    }

    async fn file_id(&self, workspace_id: i64, uri: &str) -> DbManagerResult<Option<i64>> {
        sqlx::query_scalar("SELECT id FROM files WHERE workspace_id = ? AND uri = ?")
            .bind(workspace_id)
            .bind(uri)
            .fetch_optional(&self.pool)
            .await
            .map_err(DbManagerError::database)
    }

    async fn node_exists(&self, node_id: &str) -> DbManagerResult<bool> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM nodes WHERE id = ?")
            .bind(node_id)
            .fetch_one(&self.pool)
            .await
            .map_err(DbManagerError::database)?;

        Ok(count > 0)
    }

    async fn start_run(
        &self,
        workspace_id: i64,
        provider: &str,
        provider_version: Option<&str>,
        git_commit: Option<&str>,
    ) -> DbManagerResult<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO extraction_runs (
              workspace_id,
              provider,
              provider_version,
              git_commit
            )
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(workspace_id)
        .bind(provider)
        .bind(provider_version)
        .bind(git_commit)
        .execute(&self.pool)
        .await
        .map_err(DbManagerError::database)?;

        Ok(result.last_insert_rowid())
    }

    async fn finish_run(&self, run_id: i64, status: &str) -> DbManagerResult<()> {
        sqlx::query(
            r#"
            UPDATE extraction_runs
            SET finished_at = CURRENT_TIMESTAMP,
                status = ?
            WHERE id = ?
            "#,
        )
        .bind(status)
        .bind(run_id)
        .execute(&self.pool)
        .await
        .map_err(DbManagerError::database)?;

        Ok(())
    }

    async fn upsert_file(&self, input: OwnedFileInput) -> DbManagerResult<i64> {
        let properties_json = input.properties_json.to_string();

        sqlx::query(
            r#"
            INSERT INTO files (
              workspace_id,
              uri,
              path,
              language,
              content_hash,
              last_seen_run_id,
              properties_json
            )
            VALUES (?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(workspace_id, uri) DO UPDATE SET
              path = excluded.path,
              language = excluded.language,
              content_hash = excluded.content_hash,
              last_seen_run_id = excluded.last_seen_run_id,
              properties_json = excluded.properties_json
            "#,
        )
        .bind(input.workspace_id)
        .bind(&input.uri)
        .bind(&input.path)
        .bind(&input.language)
        .bind(input.content_hash.as_deref())
        .bind(input.last_seen_run_id)
        .bind(properties_json)
        .execute(&self.pool)
        .await
        .map_err(DbManagerError::database)?;

        let id = sqlx::query_scalar("SELECT id FROM files WHERE workspace_id = ? AND uri = ?")
            .bind(input.workspace_id)
            .bind(&input.uri)
            .fetch_one(&self.pool)
            .await
            .map_err(DbManagerError::database)?;

        Ok(id)
    }

    async fn upsert_node(&self, input: OwnedNodeInput) -> DbManagerResult<String> {
        let id = node_id(input.workspace_id, &input.language, &input.symbol_key);
        let properties_json = input.properties_json.to_string();
        let range = input.range;
        let selection_range = input.selection_range;

        sqlx::query(
            r#"
            INSERT INTO nodes (
              id,
              workspace_id,
              language,
              kind,
              name,
              qualified_name,
              display_name,
              symbol_key,
              file_id,
              start_line,
              start_col,
              end_line,
              end_col,
              selection_start_line,
              selection_start_col,
              container_node_id,
              properties_json,
              first_seen_run_id,
              last_seen_run_id,
              valid_to_run_id
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NULL)
            ON CONFLICT(workspace_id, language, symbol_key) DO UPDATE SET
              kind = excluded.kind,
              name = excluded.name,
              qualified_name = excluded.qualified_name,
              display_name = excluded.display_name,
              file_id = excluded.file_id,
              start_line = excluded.start_line,
              start_col = excluded.start_col,
              end_line = excluded.end_line,
              end_col = excluded.end_col,
              selection_start_line = excluded.selection_start_line,
              selection_start_col = excluded.selection_start_col,
              container_node_id = excluded.container_node_id,
              properties_json = excluded.properties_json,
              last_seen_run_id = excluded.last_seen_run_id,
              valid_to_run_id = NULL
            "#,
        )
        .bind(&id)
        .bind(input.workspace_id)
        .bind(&input.language)
        .bind(&input.kind)
        .bind(&input.name)
        .bind(input.qualified_name.as_deref())
        .bind(input.display_name.as_deref())
        .bind(&input.symbol_key)
        .bind(input.file_id)
        .bind(range.map(|value| value.start_line))
        .bind(range.map(|value| value.start_col))
        .bind(range.map(|value| value.end_line))
        .bind(range.map(|value| value.end_col))
        .bind(selection_range.map(|value| value.start_line))
        .bind(selection_range.map(|value| value.start_col))
        .bind(input.container_node_id.as_deref())
        .bind(properties_json)
        .bind(input.run_id)
        .bind(input.run_id)
        .execute(&self.pool)
        .await
        .map_err(DbManagerError::database)?;

        let stored_id = sqlx::query_scalar(
            r#"
            SELECT id
            FROM nodes
            WHERE workspace_id = ?
              AND language = ?
              AND symbol_key = ?
            "#,
        )
        .bind(input.workspace_id)
        .bind(&input.language)
        .bind(&input.symbol_key)
        .fetch_one(&self.pool)
        .await
        .map_err(DbManagerError::database)?;

        Ok(stored_id)
    }

    async fn upsert_edge(&self, input: OwnedEdgeInput) -> DbManagerResult<String> {
        let id = edge_id(
            input.workspace_id,
            &input.src_node_id,
            &input.dst_node_id,
            &input.relation,
            input.context.as_deref(),
        );
        let properties_json = input.properties_json.to_string();

        sqlx::query(
            r#"
            INSERT INTO edges (
              id,
              workspace_id,
              src_node_id,
              dst_node_id,
              relation,
              context,
              confidence,
              confidence_score,
              weight,
              properties_json,
              first_seen_run_id,
              last_seen_run_id,
              valid_to_run_id
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NULL)
            ON CONFLICT(id) DO UPDATE SET
              confidence = excluded.confidence,
              confidence_score = excluded.confidence_score,
              weight = excluded.weight,
              properties_json = excluded.properties_json,
              last_seen_run_id = excluded.last_seen_run_id,
              valid_to_run_id = NULL
            "#,
        )
        .bind(&id)
        .bind(input.workspace_id)
        .bind(&input.src_node_id)
        .bind(&input.dst_node_id)
        .bind(&input.relation)
        .bind(input.context.as_deref())
        .bind(&input.confidence)
        .bind(input.confidence_score)
        .bind(input.weight)
        .bind(properties_json)
        .bind(input.run_id)
        .bind(input.run_id)
        .execute(&self.pool)
        .await
        .map_err(DbManagerError::database)?;

        Ok(id)
    }

    async fn insert_occurrence(&self, input: OwnedOccurrenceInput) -> DbManagerResult<i64> {
        let raw_json = input.raw_json.map(|value| value.to_string());

        let result = sqlx::query(
            r#"
            INSERT INTO occurrences (
              node_id,
              run_id,
              file_id,
              role,
              start_line,
              start_col,
              end_line,
              end_col,
              enclosing_node_id,
              raw_json
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&input.node_id)
        .bind(input.run_id)
        .bind(input.file_id)
        .bind(&input.role)
        .bind(input.range.start_line)
        .bind(input.range.start_col)
        .bind(input.range.end_line)
        .bind(input.range.end_col)
        .bind(input.enclosing_node_id.as_deref())
        .bind(raw_json)
        .execute(&self.pool)
        .await
        .map_err(DbManagerError::database)?;

        Ok(result.last_insert_rowid())
    }

    async fn insert_edge_evidence(&self, input: OwnedEdgeEvidenceInput) -> DbManagerResult<i64> {
        let raw_json = input.raw_json.map(|value| value.to_string());
        let range = input.range;

        let result = sqlx::query(
            r#"
            INSERT INTO edge_evidence (
              edge_id,
              run_id,
              provider,
              lsp_method,
              file_id,
              start_line,
              start_col,
              end_line,
              end_col,
              raw_json
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&input.edge_id)
        .bind(input.run_id)
        .bind(&input.provider)
        .bind(input.lsp_method.as_deref())
        .bind(input.file_id)
        .bind(range.map(|value| value.start_line))
        .bind(range.map(|value| value.start_col))
        .bind(range.map(|value| value.end_line))
        .bind(range.map(|value| value.end_col))
        .bind(raw_json)
        .execute(&self.pool)
        .await
        .map_err(DbManagerError::database)?;

        Ok(result.last_insert_rowid())
    }

    async fn start_route_status(&self, input: OwnedRouteStatusStartInput) -> DbManagerResult<i64> {
        let diagnostics_json = input.diagnostics_json.to_string();

        sqlx::query(
            r#"
            INSERT INTO extraction_route_status (
              workspace_id,
              route,
              scope,
              scope_key,
              file_id,
              provider,
              provider_version,
              content_hash,
              last_started_run_id,
              last_status,
              diagnostics_json,
              updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'running', ?, CURRENT_TIMESTAMP)
            ON CONFLICT(workspace_id, route, scope, scope_key, provider) DO UPDATE SET
              file_id = excluded.file_id,
              provider_version = excluded.provider_version,
              content_hash = excluded.content_hash,
              last_started_run_id = excluded.last_started_run_id,
              last_status = 'running',
              diagnostics_json = excluded.diagnostics_json,
              updated_at = CURRENT_TIMESTAMP
            "#,
        )
        .bind(input.workspace_id)
        .bind(&input.route)
        .bind(&input.scope)
        .bind(&input.scope_key)
        .bind(input.file_id)
        .bind(&input.provider)
        .bind(input.provider_version.as_deref())
        .bind(input.content_hash.as_deref())
        .bind(input.run_id)
        .bind(diagnostics_json)
        .execute(&self.pool)
        .await
        .map_err(DbManagerError::database)?;

        self.route_status_id(
            input.workspace_id,
            &input.route,
            &input.scope,
            &input.scope_key,
            &input.provider,
        )
        .await
    }

    async fn complete_route_status(
        &self,
        input: OwnedRouteStatusCompleteInput,
    ) -> DbManagerResult<()> {
        let diagnostics_json = input.diagnostics_json.to_string();

        sqlx::query(
            r#"
            UPDATE extraction_route_status
            SET provider_version = ?,
                content_hash = ?,
                last_complete_run_id = ?,
                last_status = 'complete',
                diagnostics_json = ?,
                updated_at = CURRENT_TIMESTAMP
            WHERE workspace_id = ?
              AND route = ?
              AND scope = ?
              AND scope_key = ?
              AND provider = ?
            "#,
        )
        .bind(input.provider_version.as_deref())
        .bind(input.content_hash.as_deref())
        .bind(input.run_id)
        .bind(diagnostics_json)
        .bind(input.workspace_id)
        .bind(&input.route)
        .bind(&input.scope)
        .bind(&input.scope_key)
        .bind(&input.provider)
        .execute(&self.pool)
        .await
        .map_err(DbManagerError::database)?;

        Ok(())
    }

    async fn fail_route_status(&self, input: OwnedRouteStatusFailInput) -> DbManagerResult<()> {
        let diagnostics_json = input.diagnostics_json.to_string();

        sqlx::query(
            r#"
            UPDATE extraction_route_status
            SET last_status = 'failed',
                diagnostics_json = ?,
                updated_at = CURRENT_TIMESTAMP
            WHERE workspace_id = ?
              AND route = ?
              AND scope = ?
              AND scope_key = ?
              AND provider = ?
              AND last_started_run_id = ?
            "#,
        )
        .bind(diagnostics_json)
        .bind(input.workspace_id)
        .bind(&input.route)
        .bind(&input.scope)
        .bind(&input.scope_key)
        .bind(&input.provider)
        .bind(input.run_id)
        .execute(&self.pool)
        .await
        .map_err(DbManagerError::database)?;

        Ok(())
    }

    async fn record_route_observation(
        &self,
        input: OwnedRouteObservationInput,
    ) -> DbManagerResult<()> {
        let properties_json = input.properties_json.to_string();

        sqlx::query(
            r#"
            INSERT INTO route_observations (
              workspace_id,
              run_id,
              route,
              scope,
              scope_key,
              provider,
              entity_kind,
              entity_id,
              source_file_id,
              properties_json
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(
              run_id,
              route,
              scope,
              scope_key,
              provider,
              entity_kind,
              entity_id,
              source_file_id
            ) DO UPDATE SET
              properties_json = excluded.properties_json
            "#,
        )
        .bind(input.workspace_id)
        .bind(input.run_id)
        .bind(&input.route)
        .bind(&input.scope)
        .bind(&input.scope_key)
        .bind(&input.provider)
        .bind(&input.entity_kind)
        .bind(&input.entity_id)
        .bind(input.source_file_id)
        .bind(properties_json)
        .execute(&self.pool)
        .await
        .map_err(DbManagerError::database)?;

        Ok(())
    }

    async fn close_stale_nodes_for_route(
        &self,
        input: OwnedCloseStaleRouteInput,
    ) -> DbManagerResult<u64> {
        if !self.route_completed_for_run(&input).await? {
            return Ok(0);
        }

        let result = sqlx::query(
            r#"
            UPDATE nodes
            SET valid_to_run_id = ?
            WHERE workspace_id = ?
              AND valid_to_run_id IS NULL
              AND id IN (
                SELECT DISTINCT previous.entity_id
                FROM route_observations previous
                WHERE previous.workspace_id = ?
                  AND previous.route = ?
                  AND previous.scope = ?
                  AND previous.scope_key = ?
                  AND previous.provider = ?
                  AND previous.entity_kind = 'node'
                  AND previous.run_id <> ?
                  AND NOT EXISTS (
                    SELECT 1
                    FROM route_observations current
                    WHERE current.workspace_id = previous.workspace_id
                      AND current.route = previous.route
                      AND current.scope = previous.scope
                      AND current.scope_key = previous.scope_key
                      AND current.provider = previous.provider
                      AND current.entity_kind = previous.entity_kind
                      AND current.entity_id = previous.entity_id
                      AND current.run_id = ?
                  )
              )
            "#,
        )
        .bind(input.run_id)
        .bind(input.workspace_id)
        .bind(input.workspace_id)
        .bind(&input.route)
        .bind(&input.scope)
        .bind(&input.scope_key)
        .bind(&input.provider)
        .bind(input.run_id)
        .bind(input.run_id)
        .execute(&self.pool)
        .await
        .map_err(DbManagerError::database)?;

        Ok(result.rows_affected())
    }

    async fn close_stale_edges_for_route(
        &self,
        input: OwnedCloseStaleRouteInput,
    ) -> DbManagerResult<u64> {
        if !self.route_completed_for_run(&input).await? {
            return Ok(0);
        }

        let result = sqlx::query(
            r#"
            UPDATE edges
            SET valid_to_run_id = ?
            WHERE workspace_id = ?
              AND valid_to_run_id IS NULL
              AND id IN (
                SELECT DISTINCT previous.entity_id
                FROM route_observations previous
                WHERE previous.workspace_id = ?
                  AND previous.route = ?
                  AND previous.scope = ?
                  AND previous.scope_key = ?
                  AND previous.provider = ?
                  AND previous.entity_kind = 'edge'
                  AND previous.run_id <> ?
                  AND NOT EXISTS (
                    SELECT 1
                    FROM route_observations current
                    WHERE current.workspace_id = previous.workspace_id
                      AND current.route = previous.route
                      AND current.scope = previous.scope
                      AND current.scope_key = previous.scope_key
                      AND current.provider = previous.provider
                      AND current.entity_kind = previous.entity_kind
                      AND current.entity_id = previous.entity_id
                      AND current.run_id = ?
                  )
              )
            "#,
        )
        .bind(input.run_id)
        .bind(input.workspace_id)
        .bind(input.workspace_id)
        .bind(&input.route)
        .bind(&input.scope)
        .bind(&input.scope_key)
        .bind(&input.provider)
        .bind(input.run_id)
        .bind(input.run_id)
        .execute(&self.pool)
        .await
        .map_err(DbManagerError::database)?;

        Ok(result.rows_affected())
    }

    async fn demo_seed(&self, root_uri: &str) -> DbManagerResult<DemoSeedSummary> {
        let workspace_id = self.create_workspace(root_uri, "rust").await?;
        let run_id = self
            .start_run(workspace_id, "demo", Some(env!("CARGO_PKG_VERSION")), None)
            .await?;
        let file_uri = format!("{}/src/lib.rs", root_uri.trim_end_matches('/'));
        let file_id = self
            .upsert_file(
                FileInput {
                    workspace_id,
                    uri: &file_uri,
                    path: "src/lib.rs",
                    language: "rust",
                    content_hash: None,
                    last_seen_run_id: Some(run_id),
                    properties_json: json!({}),
                }
                .into(),
            )
            .await?;

        let caller_range = TextRange {
            start_line: 1,
            start_col: 0,
            end_line: 3,
            end_col: 1,
        };
        let callee_range = TextRange {
            start_line: 5,
            start_col: 0,
            end_line: 7,
            end_col: 1,
        };
        let caller_symbol_key = format!("{file_uri}#function:caller:1:0");
        let callee_symbol_key = format!("{file_uri}#function:callee:5:0");
        let caller_node_id = self
            .upsert_node(
                NodeInput {
                    workspace_id,
                    language: "rust",
                    kind: "function",
                    name: "caller",
                    qualified_name: Some("demo::caller"),
                    display_name: Some("caller"),
                    symbol_key: &caller_symbol_key,
                    file_id: Some(file_id),
                    range: Some(caller_range),
                    selection_range: Some(TextRange {
                        start_line: 1,
                        start_col: 3,
                        end_line: 1,
                        end_col: 9,
                    }),
                    container_node_id: None,
                    properties_json: json!({}),
                    run_id: Some(run_id),
                }
                .into(),
            )
            .await?;
        let callee_node_id = self
            .upsert_node(
                NodeInput {
                    workspace_id,
                    language: "rust",
                    kind: "function",
                    name: "callee",
                    qualified_name: Some("demo::callee"),
                    display_name: Some("callee"),
                    symbol_key: &callee_symbol_key,
                    file_id: Some(file_id),
                    range: Some(callee_range),
                    selection_range: Some(TextRange {
                        start_line: 5,
                        start_col: 3,
                        end_line: 5,
                        end_col: 9,
                    }),
                    container_node_id: None,
                    properties_json: json!({}),
                    run_id: Some(run_id),
                }
                .into(),
            )
            .await?;
        let edge_id = self
            .upsert_edge(
                EdgeInput {
                    workspace_id,
                    src_node_id: &caller_node_id,
                    dst_node_id: &callee_node_id,
                    relation: "calls",
                    context: None,
                    confidence: "EXTRACTED",
                    confidence_score: 1.0,
                    weight: 1.0,
                    properties_json: json!({}),
                    run_id: Some(run_id),
                }
                .into(),
            )
            .await?;

        self.insert_occurrence(
            OccurrenceInput {
                node_id: &caller_node_id,
                run_id,
                file_id,
                role: "definition",
                range: caller_range,
                enclosing_node_id: None,
                raw_json: Some(json!({ "source": "demo-seed" })),
            }
            .into(),
        )
        .await?;
        self.insert_edge_evidence(
            EdgeEvidenceInput {
                edge_id: &edge_id,
                run_id,
                provider: "demo",
                lsp_method: Some("demo/calls"),
                file_id: Some(file_id),
                range: Some(TextRange {
                    start_line: 2,
                    start_col: 4,
                    end_line: 2,
                    end_col: 12,
                }),
                raw_json: Some(json!({ "source": "demo-seed" })),
            }
            .into(),
        )
        .await?;
        self.finish_run(run_id, "complete").await?;

        Ok(DemoSeedSummary {
            workspace_id,
            run_id,
            file_id,
            caller_node_id,
            callee_node_id,
            edge_id,
        })
    }

    async fn route_status_id(
        &self,
        workspace_id: i64,
        route: &str,
        scope: &str,
        scope_key: &str,
        provider: &str,
    ) -> DbManagerResult<i64> {
        sqlx::query_scalar(
            r#"
            SELECT id
            FROM extraction_route_status
            WHERE workspace_id = ?
              AND route = ?
              AND scope = ?
              AND scope_key = ?
              AND provider = ?
            "#,
        )
        .bind(workspace_id)
        .bind(route)
        .bind(scope)
        .bind(scope_key)
        .bind(provider)
        .fetch_one(&self.pool)
        .await
        .map_err(DbManagerError::database)
    }

    async fn route_completed_for_run(
        &self,
        input: &OwnedCloseStaleRouteInput,
    ) -> DbManagerResult<bool> {
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM extraction_route_status
            WHERE workspace_id = ?
              AND route = ?
              AND scope = ?
              AND scope_key = ?
              AND provider = ?
              AND last_complete_run_id = ?
              AND last_status = 'complete'
            "#,
        )
        .bind(input.workspace_id)
        .bind(&input.route)
        .bind(&input.scope)
        .bind(&input.scope_key)
        .bind(&input.provider)
        .bind(input.run_id)
        .fetch_one(&self.pool)
        .await
        .map_err(DbManagerError::database)?;

        Ok(count > 0)
    }
}
