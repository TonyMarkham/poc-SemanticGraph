use crate::{
    ActiveFileSymbol, ActiveFileSymbols, DbManagerError, DbManagerResult, DbWriteProgressKind,
    DemoSeedSummary, DocumentSymbolWriteBatchCloseStaleRouteInput,
    DocumentSymbolWriteBatchEdgeEvidenceInput, DocumentSymbolWriteBatchFileInput,
    DocumentSymbolWriteBatchInput, DocumentSymbolWriteBatchNodeInput,
    DocumentSymbolWriteBatchObservationInput, DocumentSymbolWriteBatchOccurrenceInput,
    DocumentSymbolWriteBatchRouteStatusCompleteInput,
    DocumentSymbolWriteBatchRouteStatusStartInput, DocumentSymbolWriteBatchSummary,
    EdgeEvidenceInput, EdgeInput, FileInput, FtsWriteBatchDocumentInput, FtsWriteBatchInput,
    FtsWriteBatchSeenDocumentInput, NodeInput, OccurrenceInput, RouteWriteBatchEdgeEvidenceInput,
    RouteWriteBatchEdgeInput, RouteWriteBatchInput, RouteWriteBatchObservationInput,
    RouteWriteBatchOccurrenceInput, StaleFileSummary, TextRange, WriteProgress, WriteSummary,
    commands::Commands,
    edge_id,
    models::{
        OwnedCloseStaleFileInput, OwnedCloseStaleFtsDocumentsInput, OwnedCloseStaleRouteInput,
        OwnedEdgeEvidenceInput, OwnedEdgeInput, OwnedFileInput, OwnedFtsDocumentInput,
        OwnedNodeInput, OwnedOccurrenceInput, OwnedRouteObservationInput,
        OwnedRouteStatusCompleteInput, OwnedRouteStatusFailInput, OwnedRouteStatusStartInput,
    },
    node_id,
};

use serde_json::json;
use sqlx::{Executor, Row, SqlitePool};
use std::collections::HashMap;
use tokio::sync::{broadcast, mpsc, oneshot};

const FILE_ROUTE_SCOPE: &str = "file";

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

fn text_range_from_row(row: &sqlx::sqlite::SqliteRow) -> Option<TextRange> {
    let start_line: Option<i64> = row.get("start_line");
    let start_col: Option<i64> = row.get("start_col");
    let end_line: Option<i64> = row.get("end_line");
    let end_col: Option<i64> = row.get("end_col");

    Some(TextRange {
        start_line: start_line?,
        start_col: start_col?,
        end_line: end_line?,
        end_col: end_col?,
    })
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
            Commands::FileRouteContentHashes {
                workspace_id,
                route,
                provider,
                response,
            } => {
                let result = self
                    .file_route_content_hashes(workspace_id, &route, &provider)
                    .await;
                let _send_result = response.send(result);
            }
            Commands::ActiveFtsDocumentHashes {
                workspace_id,
                response,
            } => {
                let result = self.active_fts_document_hashes(workspace_id).await;
                let _send_result = response.send(result);
            }
            Commands::ActiveFileSymbols {
                workspace_id,
                file_uris,
                response,
            } => {
                let result = self.active_file_symbols(workspace_id, &file_uris).await;
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
            Commands::WriteRouteBatch { input, response } => {
                let result = run_write_command!(self, self.write_route_batch(input));
                self.send_write_response(response, result).await;
            }
            Commands::WriteDocumentSymbolBatch { input, response } => {
                let result = run_write_command!(self, self.write_document_symbol_batch(input));
                self.send_write_response(response, result).await;
            }
            Commands::WriteFtsBatch { input, response } => {
                let result = run_write_command!(self, self.write_fts_batch(input));
                self.send_write_response(response, result).await;
            }
            Commands::CloseStaleNodesForRoute { input, response } => {
                let result = run_write_command!(self, self.close_stale_nodes_for_route(input));
                self.send_write_response(response, result).await;
            }
            Commands::CloseStaleFile { input, response } => {
                let result = run_write_command!(self, self.close_stale_file(input));
                self.send_write_response(response, result).await;
            }
            Commands::CloseStaleFtsDocumentsForWorkspace { input, response } => {
                let result =
                    run_write_command!(self, self.close_stale_fts_documents_for_workspace(input));
                self.send_write_response(response, result).await;
            }
            Commands::CloseStaleEdgesForRoute { input, response } => {
                let result = run_write_command!(self, self.close_stale_edges_for_route(input));
                self.send_write_response(response, result).await;
            }
            Commands::CloseStaleEdgesForRouteSourceFile { input, response } => {
                let result =
                    run_write_command!(self, self.close_stale_edges_for_route_source_file(input));
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

    async fn file_route_content_hashes(
        &self,
        workspace_id: i64,
        route: &str,
        provider: &str,
    ) -> DbManagerResult<HashMap<String, Option<String>>> {
        let rows = sqlx::query(
            r#"
            SELECT scope_key, content_hash
            FROM extraction_route_status
            WHERE workspace_id = ?
              AND route = ?
              AND scope = ?
              AND provider = ?
              AND last_status = 'complete'
              AND last_complete_run_id IS NOT NULL
            "#,
        )
        .bind(workspace_id)
        .bind(route)
        .bind(FILE_ROUTE_SCOPE)
        .bind(provider)
        .fetch_all(&self.pool)
        .await
        .map_err(DbManagerError::database)?;

        let mut hashes = HashMap::with_capacity(rows.len());
        for row in rows {
            hashes.insert(row.get("scope_key"), row.get("content_hash"));
        }

        Ok(hashes)
    }

    async fn active_fts_document_hashes(
        &self,
        workspace_id: i64,
    ) -> DbManagerResult<HashMap<String, String>> {
        let rows = sqlx::query(
            r#"
            SELECT files.uri, fts_documents.content_hash
            FROM fts_documents
            JOIN files
              ON files.id = fts_documents.file_id
             AND files.workspace_id = fts_documents.workspace_id
            WHERE fts_documents.workspace_id = ?
              AND fts_documents.valid_to_run_id IS NULL
            "#,
        )
        .bind(workspace_id)
        .fetch_all(&self.pool)
        .await
        .map_err(DbManagerError::database)?;

        let mut hashes = HashMap::with_capacity(rows.len());
        for row in rows {
            hashes.insert(row.get("uri"), row.get("content_hash"));
        }

        Ok(hashes)
    }

    async fn active_file_symbols(
        &self,
        workspace_id: i64,
        file_uris: &[String],
    ) -> DbManagerResult<Vec<ActiveFileSymbols>> {
        let mut files = Vec::with_capacity(file_uris.len());
        for file_uri in file_uris {
            let Some(file_row) = sqlx::query(
                r#"
                SELECT id, uri, path, language, content_hash, properties_json
                FROM files
                WHERE workspace_id = ?
                  AND uri = ?
                "#,
            )
            .bind(workspace_id)
            .bind(file_uri)
            .fetch_optional(&self.pool)
            .await
            .map_err(DbManagerError::database)?
            else {
                continue;
            };

            let file_id: i64 = file_row.get("id");
            let symbol_rows = sqlx::query(
                r#"
                SELECT
                  id,
                  symbol_key,
                  kind,
                  name,
                  qualified_name,
                  start_line,
                  start_col,
                  end_line,
                  end_col,
                  selection_start_line,
                  selection_start_col,
                  container_node_id,
                  properties_json
                FROM nodes
                WHERE workspace_id = ?
                  AND file_id = ?
                  AND valid_to_run_id IS NULL
                  AND kind <> 'file'
                ORDER BY
                  COALESCE(start_line, 0),
                  COALESCE(start_col, 0),
                  COALESCE(selection_start_line, 0),
                  COALESCE(selection_start_col, 0),
                  name
                "#,
            )
            .bind(workspace_id)
            .bind(file_id)
            .fetch_all(&self.pool)
            .await
            .map_err(DbManagerError::database)?;

            let mut symbols = Vec::with_capacity(symbol_rows.len());
            for row in symbol_rows {
                symbols.push(ActiveFileSymbol {
                    node_id: row.get("id"),
                    symbol_key: row.get("symbol_key"),
                    kind: row.get("kind"),
                    name: row.get("name"),
                    qualified_name: row.get("qualified_name"),
                    range: text_range_from_row(&row),
                    selection_range: None,
                    container_node_id: row.get("container_node_id"),
                    properties_json: row.get("properties_json"),
                });
            }

            files.push(ActiveFileSymbols {
                uri: file_row.get("uri"),
                relative_path: file_row.get("path"),
                language: file_row.get("language"),
                content_hash: file_row.get("content_hash"),
                properties_json: file_row.get("properties_json"),
                symbols,
            });
        }

        Ok(files)
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

    async fn upsert_fts_document_metadata(
        &self,
        input: &OwnedFtsDocumentInput,
    ) -> DbManagerResult<i64> {
        let properties_json = input.properties_json.to_string();

        sqlx::query(
            r#"
            INSERT INTO fts_documents (
              workspace_id,
              file_id,
              language,
              content_hash,
              byte_len,
              first_seen_run_id,
              last_seen_run_id,
              valid_to_run_id,
              properties_json
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, NULL, ?)
            ON CONFLICT(workspace_id, file_id) DO UPDATE SET
              language = excluded.language,
              content_hash = excluded.content_hash,
              byte_len = excluded.byte_len,
              indexed_at = CURRENT_TIMESTAMP,
              last_seen_run_id = excluded.last_seen_run_id,
              valid_to_run_id = NULL,
              properties_json = excluded.properties_json
            "#,
        )
        .bind(input.workspace_id)
        .bind(input.file_id)
        .bind(&input.language)
        .bind(&input.content_hash)
        .bind(input.byte_len)
        .bind(input.run_id)
        .bind(input.run_id)
        .bind(properties_json)
        .execute(&self.pool)
        .await
        .map_err(DbManagerError::database)?;

        let document_id: i64 = sqlx::query_scalar(
            "SELECT id FROM fts_documents WHERE workspace_id = ? AND file_id = ?",
        )
        .bind(input.workspace_id)
        .bind(input.file_id)
        .fetch_one(&self.pool)
        .await
        .map_err(DbManagerError::database)?;

        Ok(document_id)
    }

    async fn upsert_fts_document_content(
        &self,
        document_id: i64,
        input: &OwnedFtsDocumentInput,
    ) -> DbManagerResult<()> {
        sqlx::query(
            r#"
            INSERT INTO fts_document_contents (
              document_id,
              file_id,
              path,
              language,
              content,
              updated_at
            )
            VALUES (?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
            ON CONFLICT(document_id) DO UPDATE SET
              file_id = excluded.file_id,
              path = excluded.path,
              language = excluded.language,
              content = excluded.content,
              updated_at = CURRENT_TIMESTAMP
            "#,
        )
        .bind(document_id)
        .bind(input.file_id)
        .bind(&input.path)
        .bind(&input.language)
        .bind(&input.content)
        .execute(&self.pool)
        .await
        .map_err(DbManagerError::database)?;

        Ok(())
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

    async fn upsert_node_without_select(&self, input: OwnedNodeInput) -> DbManagerResult<()> {
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

        Ok(())
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

    async fn insert_occurrence_without_rowid(
        &self,
        input: OwnedOccurrenceInput,
    ) -> DbManagerResult<()> {
        let raw_json = input.raw_json.map(|value| value.to_string());

        sqlx::query(
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

        Ok(())
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

    async fn insert_edge_evidence_without_rowid(
        &self,
        input: OwnedEdgeEvidenceInput,
    ) -> DbManagerResult<()> {
        let raw_json = input.raw_json.map(|value| value.to_string());
        let range = input.range;

        sqlx::query(
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

        Ok(())
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

    async fn start_route_status_without_select(
        &self,
        input: OwnedRouteStatusStartInput,
    ) -> DbManagerResult<()> {
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

        Ok(())
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

    async fn write_route_batch(&self, input: RouteWriteBatchInput) -> DbManagerResult<()> {
        for edge in input.edges {
            self.upsert_edge(owned_edge_input(edge)).await?;
        }
        for occurrence in input.occurrences {
            self.insert_occurrence(owned_occurrence_input(occurrence))
                .await?;
        }
        for edge_evidence in input.edge_evidence {
            self.insert_edge_evidence(owned_edge_evidence_input(edge_evidence))
                .await?;
        }
        for route_observation in input.route_observations {
            self.record_route_observation(owned_route_observation_input(route_observation))
                .await?;
        }

        Ok(())
    }

    async fn write_fts_batch(&self, input: FtsWriteBatchInput) -> DbManagerResult<()> {
        for seen_document in input.seen_documents {
            self.mark_fts_document_seen(seen_document).await?;
        }
        for document in input.documents {
            self.upsert_fts_batch_document(document).await?;
        }

        Ok(())
    }

    async fn mark_fts_document_seen(
        &self,
        input: FtsWriteBatchSeenDocumentInput,
    ) -> DbManagerResult<()> {
        sqlx::query(
            r#"
            UPDATE files
            SET content_hash = ?,
                last_seen_run_id = ?
            WHERE workspace_id = ?
              AND uri = ?
            "#,
        )
        .bind(&input.content_hash)
        .bind(input.run_id)
        .bind(input.workspace_id)
        .bind(&input.uri)
        .execute(&self.pool)
        .await
        .map_err(DbManagerError::database)?;

        sqlx::query(
            r#"
            UPDATE fts_documents
            SET last_seen_run_id = ?,
                valid_to_run_id = NULL
            WHERE workspace_id = ?
              AND content_hash = ?
              AND file_id = (
                SELECT id
                FROM files
                WHERE workspace_id = ?
                  AND uri = ?
              )
            "#,
        )
        .bind(input.run_id)
        .bind(input.workspace_id)
        .bind(&input.content_hash)
        .bind(input.workspace_id)
        .bind(&input.uri)
        .execute(&self.pool)
        .await
        .map_err(DbManagerError::database)?;

        Ok(())
    }

    async fn upsert_fts_batch_document(
        &self,
        input: FtsWriteBatchDocumentInput,
    ) -> DbManagerResult<i64> {
        let file_id = self
            .upsert_file(OwnedFileInput {
                workspace_id: input.workspace_id,
                uri: input.uri,
                path: input.path.clone(),
                language: input.language.clone(),
                content_hash: Some(input.content_hash.clone()),
                last_seen_run_id: Some(input.run_id),
                properties_json: input.properties_json.clone(),
            })
            .await?;

        let document = OwnedFtsDocumentInput {
            workspace_id: input.workspace_id,
            file_id,
            path: input.path,
            language: input.language,
            content_hash: input.content_hash,
            byte_len: input.byte_len,
            run_id: input.run_id,
            content: input.content,
            properties_json: input.properties_json,
        };
        let document_id = self.upsert_fts_document_metadata(&document).await?;
        self.upsert_fts_document_content(document_id, &document)
            .await?;

        Ok(document_id)
    }

    async fn write_document_symbol_batch(
        &self,
        input: DocumentSymbolWriteBatchInput,
    ) -> DbManagerResult<DocumentSymbolWriteBatchSummary> {
        let mut file_ids = HashMap::new();
        for file in input.files {
            let uri = file.uri.clone();
            let file_id = self.upsert_file(owned_file_input(file)).await?;
            file_ids.insert(uri, file_id);
        }

        for route_status in input.route_status_starts {
            self.start_route_status_without_select(owned_document_symbol_route_status_start_input(
                route_status,
                &file_ids,
            )?)
            .await?;
        }
        for node in input.nodes {
            self.upsert_node_without_select(owned_document_symbol_node_input(node, &file_ids)?)
                .await?;
        }
        for occurrence in input.occurrences {
            self.insert_occurrence_without_rowid(owned_document_symbol_occurrence_input(
                occurrence, &file_ids,
            )?)
            .await?;
        }
        for edge in input.edges {
            self.upsert_edge(owned_edge_input(edge)).await?;
        }
        for edge_evidence in input.edge_evidence {
            self.insert_edge_evidence_without_rowid(owned_document_symbol_edge_evidence_input(
                edge_evidence,
                &file_ids,
            )?)
            .await?;
        }
        for route_observation in input.route_observations {
            self.record_route_observation(owned_document_symbol_route_observation_input(
                route_observation,
                &file_ids,
            )?)
            .await?;
        }
        for route_status in input.route_status_completes {
            self.complete_route_status(owned_document_symbol_route_status_complete_input(
                route_status,
            ))
            .await?;
        }

        self.close_stale_document_symbol_batch(&input.close_stale_nodes, &input.close_stale_edges)
            .await
    }

    async fn close_stale_document_symbol_batch(
        &self,
        close_stale_nodes: &[DocumentSymbolWriteBatchCloseStaleRouteInput],
        close_stale_edges: &[DocumentSymbolWriteBatchCloseStaleRouteInput],
    ) -> DbManagerResult<DocumentSymbolWriteBatchSummary> {
        if close_stale_nodes.is_empty() && close_stale_edges.is_empty() {
            return Ok(DocumentSymbolWriteBatchSummary::default());
        }

        self.reset_document_symbol_close_stale_requests().await?;
        for close_stale in close_stale_nodes {
            self.insert_document_symbol_close_stale_request("node", close_stale)
                .await?;
        }
        for close_stale in close_stale_edges {
            self.insert_document_symbol_close_stale_request("edge", close_stale)
                .await?;
        }

        let stale_nodes_closed = self.close_stale_document_symbol_batch_nodes().await?;
        let stale_edges_closed = self.close_stale_document_symbol_batch_edges().await?;
        self.clear_document_symbol_close_stale_requests().await?;

        Ok(DocumentSymbolWriteBatchSummary {
            stale_nodes_closed,
            stale_edges_closed,
        })
    }

    async fn reset_document_symbol_close_stale_requests(&self) -> DbManagerResult<()> {
        sqlx::query(
            r#"
            CREATE TEMP TABLE IF NOT EXISTS document_symbol_close_stale_requests (
              target_kind TEXT NOT NULL,
              workspace_id INTEGER NOT NULL,
              run_id INTEGER NOT NULL,
              route TEXT NOT NULL,
              scope TEXT NOT NULL,
              scope_key TEXT NOT NULL,
              provider TEXT NOT NULL,
              file_id INTEGER,
              PRIMARY KEY (
                target_kind,
                workspace_id,
                run_id,
                route,
                scope,
                scope_key,
                provider
              )
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(DbManagerError::database)?;

        self.clear_document_symbol_close_stale_requests().await
    }

    async fn clear_document_symbol_close_stale_requests(&self) -> DbManagerResult<()> {
        sqlx::query("DELETE FROM document_symbol_close_stale_requests")
            .execute(&self.pool)
            .await
            .map_err(DbManagerError::database)?;

        Ok(())
    }

    async fn insert_document_symbol_close_stale_request(
        &self,
        target_kind: &str,
        input: &DocumentSymbolWriteBatchCloseStaleRouteInput,
    ) -> DbManagerResult<()> {
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO document_symbol_close_stale_requests (
              target_kind,
              workspace_id,
              run_id,
              route,
              scope,
              scope_key,
              provider,
              file_id
            )
            VALUES (
              ?,
              ?,
              ?,
              ?,
              ?,
              ?,
              ?,
              (
                SELECT id
                FROM files
                WHERE workspace_id = ?
                  AND uri = ?
              )
            )
            "#,
        )
        .bind(target_kind)
        .bind(input.workspace_id)
        .bind(input.run_id)
        .bind(&input.route)
        .bind(&input.scope)
        .bind(&input.scope_key)
        .bind(&input.provider)
        .bind(input.workspace_id)
        .bind(&input.scope_key)
        .execute(&self.pool)
        .await
        .map_err(DbManagerError::database)?;

        Ok(())
    }

    async fn close_stale_document_symbol_batch_nodes(&self) -> DbManagerResult<u64> {
        let result = sqlx::query(
            r#"
            WITH completed_requests AS (
              SELECT DISTINCT
                request.workspace_id,
                request.run_id,
                request.file_id
              FROM document_symbol_close_stale_requests request
              JOIN extraction_route_status status
                ON status.workspace_id = request.workspace_id
               AND status.route = request.route
               AND status.scope = request.scope
               AND status.scope_key = request.scope_key
               AND status.provider = request.provider
               AND status.last_complete_run_id = request.run_id
               AND status.last_status = 'complete'
              WHERE request.target_kind = 'node'
                AND request.scope = ?
                AND request.file_id IS NOT NULL
            )
            UPDATE nodes
            SET valid_to_run_id = (
              SELECT completed_requests.run_id
              FROM completed_requests
              WHERE completed_requests.workspace_id = nodes.workspace_id
                AND completed_requests.file_id = nodes.file_id
              LIMIT 1
            )
            WHERE valid_to_run_id IS NULL
              AND EXISTS (
                SELECT 1
                FROM completed_requests
                WHERE completed_requests.workspace_id = nodes.workspace_id
                  AND completed_requests.file_id = nodes.file_id
                  AND (
                    nodes.last_seen_run_id IS NULL
                    OR nodes.last_seen_run_id <> completed_requests.run_id
                  )
              )
            "#,
        )
        .bind(FILE_ROUTE_SCOPE)
        .execute(&self.pool)
        .await
        .map_err(DbManagerError::database)?;

        Ok(result.rows_affected())
    }

    async fn close_stale_document_symbol_batch_edges(&self) -> DbManagerResult<u64> {
        let result = sqlx::query(
            r#"
            WITH completed_requests AS (
              SELECT DISTINCT
                request.workspace_id,
                request.run_id,
                request.file_id
              FROM document_symbol_close_stale_requests request
              JOIN extraction_route_status status
                ON status.workspace_id = request.workspace_id
               AND status.route = request.route
               AND status.scope = request.scope
               AND status.scope_key = request.scope_key
               AND status.provider = request.provider
               AND status.last_complete_run_id = request.run_id
               AND status.last_status = 'complete'
              WHERE request.target_kind = 'edge'
                AND request.scope = ?
                AND request.file_id IS NOT NULL
            ),
            stale_edges AS (
              SELECT DISTINCT
                edges.workspace_id,
                edges.id AS entity_id,
                completed_requests.run_id
              FROM edges
              JOIN nodes src
                ON src.id = edges.src_node_id
              JOIN nodes dst
                ON dst.id = edges.dst_node_id
              JOIN completed_requests
                ON completed_requests.workspace_id = edges.workspace_id
              WHERE edges.valid_to_run_id IS NULL
                AND edges.relation = 'contains'
                AND (
                  src.file_id = completed_requests.file_id
                  OR dst.file_id = completed_requests.file_id
                )
                AND (
                  edges.last_seen_run_id IS NULL
                  OR edges.last_seen_run_id <> completed_requests.run_id
                )
            )
            UPDATE edges
            SET valid_to_run_id = (
              SELECT stale_edges.run_id
              FROM stale_edges
              WHERE stale_edges.workspace_id = edges.workspace_id
                AND stale_edges.entity_id = edges.id
              LIMIT 1
            )
            WHERE valid_to_run_id IS NULL
              AND EXISTS (
                SELECT 1
                FROM stale_edges
                WHERE stale_edges.workspace_id = edges.workspace_id
                  AND stale_edges.entity_id = edges.id
              )
            "#,
        )
        .bind(FILE_ROUTE_SCOPE)
        .execute(&self.pool)
        .await
        .map_err(DbManagerError::database)?;

        Ok(result.rows_affected())
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

        let source_file_rows = self.close_stale_source_file_nodes_for_route(&input).await?;

        Ok(result.rows_affected() + source_file_rows)
    }

    async fn close_stale_file(
        &self,
        input: OwnedCloseStaleFileInput,
    ) -> DbManagerResult<StaleFileSummary> {
        let file_id = self.file_id(input.workspace_id, &input.file_uri).await?;
        let Some(file_id) = file_id else {
            return Ok(StaleFileSummary {
                file_id: None,
                stale_nodes_closed: 0,
                stale_edges_closed: 0,
            });
        };

        let edge_result = sqlx::query(
            r#"
            UPDATE edges
            SET valid_to_run_id = ?
            WHERE workspace_id = ?
              AND valid_to_run_id IS NULL
              AND (
                src_node_id IN (
                  SELECT id
                  FROM nodes
                  WHERE workspace_id = ?
                    AND file_id = ?
                )
                OR dst_node_id IN (
                  SELECT id
                  FROM nodes
                  WHERE workspace_id = ?
                    AND file_id = ?
                )
                OR id IN (
                  SELECT edge_id
                  FROM edge_evidence
                  WHERE file_id = ?
                )
                OR id IN (
                  SELECT entity_id
                  FROM route_observations
                  WHERE workspace_id = ?
                    AND entity_kind = 'edge'
                    AND source_file_id = ?
                )
              )
            "#,
        )
        .bind(input.run_id)
        .bind(input.workspace_id)
        .bind(input.workspace_id)
        .bind(file_id)
        .bind(input.workspace_id)
        .bind(file_id)
        .bind(file_id)
        .bind(input.workspace_id)
        .bind(file_id)
        .execute(&self.pool)
        .await
        .map_err(DbManagerError::database)?;

        let node_result = sqlx::query(
            r#"
            UPDATE nodes
            SET valid_to_run_id = ?
            WHERE workspace_id = ?
              AND file_id = ?
              AND valid_to_run_id IS NULL
            "#,
        )
        .bind(input.run_id)
        .bind(input.workspace_id)
        .bind(file_id)
        .execute(&self.pool)
        .await
        .map_err(DbManagerError::database)?;

        Ok(StaleFileSummary {
            file_id: Some(file_id),
            stale_nodes_closed: node_result.rows_affected(),
            stale_edges_closed: edge_result.rows_affected(),
        })
    }

    async fn close_stale_fts_documents_for_workspace(
        &self,
        input: OwnedCloseStaleFtsDocumentsInput,
    ) -> DbManagerResult<u64> {
        if !self.fts_route_completed_for_run(&input).await? {
            return Ok(0);
        }

        let stale_document_ids = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT id
            FROM fts_documents
            WHERE workspace_id = ?
              AND valid_to_run_id IS NULL
              AND (last_seen_run_id IS NULL OR last_seen_run_id < ?)
            "#,
        )
        .bind(input.workspace_id)
        .bind(input.run_id)
        .fetch_all(&self.pool)
        .await
        .map_err(DbManagerError::database)?;

        if stale_document_ids.is_empty() {
            return Ok(0);
        }

        sqlx::query(
            r#"
            UPDATE fts_documents
            SET valid_to_run_id = ?
            WHERE workspace_id = ?
              AND valid_to_run_id IS NULL
              AND (last_seen_run_id IS NULL OR last_seen_run_id < ?)
            "#,
        )
        .bind(input.run_id)
        .bind(input.workspace_id)
        .bind(input.run_id)
        .execute(&self.pool)
        .await
        .map_err(DbManagerError::database)?;

        for document_id in &stale_document_ids {
            sqlx::query("DELETE FROM fts_document_contents WHERE document_id = ?")
                .bind(document_id)
                .execute(&self.pool)
                .await
                .map_err(DbManagerError::database)?;
        }

        Ok(stale_document_ids.len() as u64)
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

        let source_file_rows = self
            .close_stale_file_related_edges_for_route(&input)
            .await?;

        Ok(result.rows_affected() + source_file_rows)
    }

    async fn close_stale_edges_for_route_source_file(
        &self,
        input: OwnedCloseStaleRouteInput,
    ) -> DbManagerResult<u64> {
        if input.scope != FILE_ROUTE_SCOPE || !self.route_completed_for_run(&input).await? {
            return Ok(0);
        }

        let Some(file_id) = self.file_id(input.workspace_id, &input.scope_key).await? else {
            return Ok(0);
        };

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
                  AND previous.provider = ?
                  AND previous.entity_kind = 'edge'
                  AND previous.source_file_id = ?
                  AND previous.run_id <> ?
                  AND NOT EXISTS (
                    SELECT 1
                    FROM route_observations current
                    WHERE current.workspace_id = previous.workspace_id
                      AND current.route = previous.route
                      AND current.scope = ?
                      AND current.scope_key = ?
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
        .bind(&input.provider)
        .bind(file_id)
        .bind(input.run_id)
        .bind(&input.scope)
        .bind(&input.scope_key)
        .bind(input.run_id)
        .execute(&self.pool)
        .await
        .map_err(DbManagerError::database)?;

        Ok(result.rows_affected())
    }

    async fn close_stale_source_file_nodes_for_route(
        &self,
        input: &OwnedCloseStaleRouteInput,
    ) -> DbManagerResult<u64> {
        if input.scope != FILE_ROUTE_SCOPE {
            return Ok(0);
        }

        let Some(file_id) = self.file_id(input.workspace_id, &input.scope_key).await? else {
            return Ok(0);
        };

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
                  AND previous.provider = ?
                  AND previous.entity_kind = 'node'
                  AND previous.source_file_id = ?
                  AND previous.run_id <> ?
                  AND NOT EXISTS (
                    SELECT 1
                    FROM route_observations current
                    WHERE current.workspace_id = previous.workspace_id
                      AND current.route = previous.route
                      AND current.scope = ?
                      AND current.scope_key = ?
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
        .bind(&input.provider)
        .bind(file_id)
        .bind(input.run_id)
        .bind(&input.scope)
        .bind(&input.scope_key)
        .bind(input.run_id)
        .execute(&self.pool)
        .await
        .map_err(DbManagerError::database)?;

        Ok(result.rows_affected())
    }

    async fn close_stale_file_related_edges_for_route(
        &self,
        input: &OwnedCloseStaleRouteInput,
    ) -> DbManagerResult<u64> {
        if input.scope != FILE_ROUTE_SCOPE {
            return Ok(0);
        }

        let Some(file_id) = self.file_id(input.workspace_id, &input.scope_key).await? else {
            return Ok(0);
        };

        let result = sqlx::query(
            r#"
            UPDATE edges
            SET valid_to_run_id = ?
            WHERE workspace_id = ?
              AND valid_to_run_id IS NULL
              AND id IN (
                SELECT DISTINCT previous.entity_id
                FROM route_observations previous
                JOIN edges previous_edge
                  ON previous_edge.id = previous.entity_id
                 AND previous_edge.workspace_id = previous.workspace_id
                LEFT JOIN nodes previous_src
                  ON previous_src.id = previous_edge.src_node_id
                LEFT JOIN nodes previous_dst
                  ON previous_dst.id = previous_edge.dst_node_id
                WHERE previous.workspace_id = ?
                  AND previous.route = ?
                  AND previous.provider = ?
                  AND previous.entity_kind = 'edge'
                  AND (
                    previous.source_file_id = ?
                    OR previous_src.file_id = ?
                    OR previous_dst.file_id = ?
                  )
                  AND previous.run_id <> ?
                  AND NOT EXISTS (
                    SELECT 1
                    FROM route_observations current
                    WHERE current.workspace_id = previous.workspace_id
                      AND current.route = previous.route
                      AND current.scope = ?
                      AND current.scope_key = ?
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
        .bind(&input.provider)
        .bind(file_id)
        .bind(file_id)
        .bind(file_id)
        .bind(input.run_id)
        .bind(&input.scope)
        .bind(&input.scope_key)
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

    async fn fts_route_completed_for_run(
        &self,
        input: &OwnedCloseStaleFtsDocumentsInput,
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

fn owned_edge_input(input: RouteWriteBatchEdgeInput) -> OwnedEdgeInput {
    OwnedEdgeInput {
        workspace_id: input.workspace_id,
        src_node_id: input.src_node_id,
        dst_node_id: input.dst_node_id,
        relation: input.relation,
        context: input.context,
        confidence: input.confidence,
        confidence_score: input.confidence_score,
        weight: input.weight,
        properties_json: input.properties_json,
        run_id: input.run_id,
    }
}

fn owned_file_input(input: DocumentSymbolWriteBatchFileInput) -> OwnedFileInput {
    OwnedFileInput {
        workspace_id: input.workspace_id,
        uri: input.uri,
        path: input.path,
        language: input.language,
        content_hash: input.content_hash,
        last_seen_run_id: input.last_seen_run_id,
        properties_json: input.properties_json,
    }
}

fn owned_document_symbol_node_input(
    input: DocumentSymbolWriteBatchNodeInput,
    file_ids: &HashMap<String, i64>,
) -> DbManagerResult<OwnedNodeInput> {
    let file_id = input
        .file_uri
        .as_deref()
        .map(|file_uri| file_id_for_uri(file_ids, file_uri))
        .transpose()?;

    Ok(OwnedNodeInput {
        workspace_id: input.workspace_id,
        language: input.language,
        kind: input.kind,
        name: input.name,
        qualified_name: input.qualified_name,
        display_name: input.display_name,
        symbol_key: input.symbol_key,
        file_id,
        range: input.range,
        selection_range: input.selection_range,
        container_node_id: input.container_node_id,
        properties_json: input.properties_json,
        run_id: input.run_id,
    })
}

fn owned_document_symbol_occurrence_input(
    input: DocumentSymbolWriteBatchOccurrenceInput,
    file_ids: &HashMap<String, i64>,
) -> DbManagerResult<OwnedOccurrenceInput> {
    Ok(OwnedOccurrenceInput {
        node_id: input.node_id,
        run_id: input.run_id,
        file_id: file_id_for_uri(file_ids, &input.file_uri)?,
        role: input.role,
        range: input.range,
        enclosing_node_id: input.enclosing_node_id,
        raw_json: input.raw_json,
    })
}

fn owned_document_symbol_edge_evidence_input(
    input: DocumentSymbolWriteBatchEdgeEvidenceInput,
    file_ids: &HashMap<String, i64>,
) -> DbManagerResult<OwnedEdgeEvidenceInput> {
    let file_id = input
        .file_uri
        .as_deref()
        .map(|file_uri| file_id_for_uri(file_ids, file_uri))
        .transpose()?;

    Ok(OwnedEdgeEvidenceInput {
        edge_id: input.edge_id,
        run_id: input.run_id,
        provider: input.provider,
        lsp_method: input.lsp_method,
        file_id,
        range: input.range,
        raw_json: input.raw_json,
    })
}

fn owned_document_symbol_route_observation_input(
    input: DocumentSymbolWriteBatchObservationInput,
    file_ids: &HashMap<String, i64>,
) -> DbManagerResult<OwnedRouteObservationInput> {
    let source_file_id = input
        .source_file_uri
        .as_deref()
        .map(|file_uri| file_id_for_uri(file_ids, file_uri))
        .transpose()?;

    Ok(OwnedRouteObservationInput {
        workspace_id: input.workspace_id,
        run_id: input.run_id,
        route: input.route,
        scope: input.scope,
        scope_key: input.scope_key,
        provider: input.provider,
        entity_kind: input.entity_kind,
        entity_id: input.entity_id,
        source_file_id,
        properties_json: input.properties_json,
    })
}

fn owned_document_symbol_route_status_start_input(
    input: DocumentSymbolWriteBatchRouteStatusStartInput,
    file_ids: &HashMap<String, i64>,
) -> DbManagerResult<OwnedRouteStatusStartInput> {
    let file_id = input
        .file_uri
        .as_deref()
        .map(|file_uri| file_id_for_uri(file_ids, file_uri))
        .transpose()?;

    Ok(OwnedRouteStatusStartInput {
        workspace_id: input.workspace_id,
        route: input.route,
        scope: input.scope,
        scope_key: input.scope_key,
        file_id,
        provider: input.provider,
        provider_version: input.provider_version,
        content_hash: input.content_hash,
        run_id: input.run_id,
        diagnostics_json: input.diagnostics_json,
    })
}

fn owned_document_symbol_route_status_complete_input(
    input: DocumentSymbolWriteBatchRouteStatusCompleteInput,
) -> OwnedRouteStatusCompleteInput {
    OwnedRouteStatusCompleteInput {
        workspace_id: input.workspace_id,
        route: input.route,
        scope: input.scope,
        scope_key: input.scope_key,
        provider: input.provider,
        provider_version: input.provider_version,
        content_hash: input.content_hash,
        run_id: input.run_id,
        diagnostics_json: input.diagnostics_json,
    }
}

fn owned_occurrence_input(input: RouteWriteBatchOccurrenceInput) -> OwnedOccurrenceInput {
    OwnedOccurrenceInput {
        node_id: input.node_id,
        run_id: input.run_id,
        file_id: input.file_id,
        role: input.role,
        range: input.range,
        enclosing_node_id: input.enclosing_node_id,
        raw_json: input.raw_json,
    }
}

fn owned_edge_evidence_input(input: RouteWriteBatchEdgeEvidenceInput) -> OwnedEdgeEvidenceInput {
    OwnedEdgeEvidenceInput {
        edge_id: input.edge_id,
        run_id: input.run_id,
        provider: input.provider,
        lsp_method: input.lsp_method,
        file_id: input.file_id,
        range: input.range,
        raw_json: input.raw_json,
    }
}

fn owned_route_observation_input(
    input: RouteWriteBatchObservationInput,
) -> OwnedRouteObservationInput {
    OwnedRouteObservationInput {
        workspace_id: input.workspace_id,
        run_id: input.run_id,
        route: input.route,
        scope: input.scope,
        scope_key: input.scope_key,
        provider: input.provider,
        entity_kind: input.entity_kind,
        entity_id: input.entity_id,
        source_file_id: input.source_file_id,
        properties_json: input.properties_json,
    }
}

fn file_id_for_uri(file_ids: &HashMap<String, i64>, file_uri: &str) -> DbManagerResult<i64> {
    file_ids.get(file_uri).copied().ok_or_else(|| {
        DbManagerError::invalid_input(format!(
            "document symbol write batch referenced file URI {file_uri} before upserting it"
        ))
    })
}
