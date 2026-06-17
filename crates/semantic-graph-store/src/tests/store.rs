use std::env;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{GraphStore, GraphStoreResult, GraphStoreStats};
use semantic_graph_db_manager::{
    CloseStaleRouteInput, DbManagerResult, DemoSeedSummary, EdgeInput, FileInput, NodeInput,
    RouteObservationInput, RouteStatusCompleteInput, RouteStatusFailInput, RouteStatusStartInput,
    TextRange, WriteHandle, WriteManager, edge_id, node_id,
};
use serde_json::json;
use sqlx::SqlitePool;

const PROVIDER: &str = "rust-analyzer";
const ROUTE_DOCUMENT_SYMBOLS: &str = "rust.document_symbols";
const ROUTE_REFERENCES: &str = "rust.references";
const ROUTE_CALLS: &str = "rust.calls";
const SCOPE_FILE: &str = "file";
const SCOPE_WORKSPACE: &str = "workspace";

fn temp_db_path() -> Result<PathBuf, Box<dyn Error>> {
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    Ok(env::temp_dir().join(format!(
        "poc-semanticgraph-store-{}-{stamp}.db",
        std::process::id()
    )))
}

struct TestStore {
    writer: WriteHandle,
    path: PathBuf,
}

impl TestStore {
    async fn create_workspace(&self, root_uri: &str, kind: &str) -> DbManagerResult<i64> {
        self.writer.create_workspace(root_uri, kind).await
    }

    async fn start_run(
        &self,
        workspace_id: i64,
        provider: &str,
        provider_version: Option<&str>,
        git_commit: Option<&str>,
    ) -> DbManagerResult<i64> {
        self.writer
            .start_run(workspace_id, provider, provider_version, git_commit)
            .await
    }

    async fn upsert_file(&self, input: FileInput<'_>) -> DbManagerResult<i64> {
        self.writer.upsert_file(input).await
    }

    async fn upsert_node(&self, input: NodeInput<'_>) -> DbManagerResult<String> {
        self.writer.upsert_node(input).await
    }

    async fn upsert_edge(&self, input: EdgeInput<'_>) -> DbManagerResult<String> {
        self.writer.upsert_edge(input).await
    }

    async fn start_route_status(&self, input: RouteStatusStartInput<'_>) -> DbManagerResult<i64> {
        self.writer.start_route_status(input).await
    }

    async fn complete_route_status(
        &self,
        input: RouteStatusCompleteInput<'_>,
    ) -> DbManagerResult<()> {
        self.writer.complete_route_status(input).await
    }

    async fn fail_route_status(&self, input: RouteStatusFailInput<'_>) -> DbManagerResult<()> {
        self.writer.fail_route_status(input).await
    }

    async fn record_route_observation(
        &self,
        input: RouteObservationInput<'_>,
    ) -> DbManagerResult<()> {
        self.writer.record_route_observation(input).await
    }

    async fn close_stale_nodes_for_route(
        &self,
        input: CloseStaleRouteInput<'_>,
    ) -> DbManagerResult<u64> {
        self.writer.close_stale_nodes_for_route(input).await
    }

    async fn close_stale_edges_for_route(
        &self,
        input: CloseStaleRouteInput<'_>,
    ) -> DbManagerResult<u64> {
        self.writer.close_stale_edges_for_route(input).await
    }

    async fn demo_seed(&self, root_uri: &str) -> DbManagerResult<DemoSeedSummary> {
        self.writer.demo_seed(root_uri).await
    }

    async fn stats(&self) -> GraphStoreResult<GraphStoreStats> {
        GraphStore::connect(&self.path).await?.stats().await
    }
}

async fn migrated_store() -> Result<TestStore, Box<dyn Error>> {
    let (store, _path) = migrated_store_with_path().await?;
    Ok(store)
}

async fn migrated_store_with_path() -> Result<(TestStore, PathBuf), Box<dyn Error>> {
    let path = temp_db_path()?;
    let writer = WriteManager::start(&path).await?;
    writer.migrate().await?;
    let store = TestStore {
        writer,
        path: path.clone(),
    };
    Ok((store, path))
}

#[tokio::test]
async fn migration_creates_empty_core_schema() -> Result<(), Box<dyn Error>> {
    let path = temp_db_path()?;
    let writer = WriteManager::start(&path).await?;
    writer.migrate().await?;
    writer.shutdown().await?;
    let store = GraphStore::connect(&path).await?;

    assert_eq!(
        store.stats().await?,
        GraphStoreStats {
            workspaces: 0,
            extraction_runs: 0,
            files: 0,
            nodes: 0,
            edges: 0,
            occurrences: 0,
            edge_evidence: 0,
        }
    );

    let pool = SqlitePool::connect(&format!("sqlite://{}", path.display())).await?;
    let index_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM sqlite_master
        WHERE type = 'index'
          AND name IN (
            'idx_files_workspace_path',
            'idx_nodes_workspace_qname',
            'idx_nodes_file',
            'idx_edges_src',
            'idx_edges_dst',
            'idx_edges_relation',
            'idx_occurrences_node_role',
            'idx_occurrences_file',
            'idx_edge_evidence_edge',
            'idx_extraction_route_status_workspace_route',
            'idx_route_observations_route_run',
            'idx_route_observations_entity',
            'idx_fts_documents_workspace_active',
            'idx_fts_documents_file'
          )
        "#,
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(index_count, 14);

    let route_table_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM sqlite_master
        WHERE type = 'table'
          AND name IN ('extraction_route_status', 'route_observations')
        "#,
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(route_table_count, 2);

    let node_search_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM sqlite_master
        WHERE type = 'table'
          AND name = 'node_search'
        "#,
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(node_search_count, 1);

    let fts_table_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM sqlite_master
        WHERE type = 'table'
          AND name IN ('fts_documents', 'fts_document_trigram_ci')
        "#,
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(fts_table_count, 2);
    let trigram_sql: String = sqlx::query_scalar(
        r#"
        SELECT sql
        FROM sqlite_master
        WHERE type = 'table'
          AND name = 'fts_document_trigram_ci'
        "#,
    )
    .fetch_one(&pool)
    .await?;
    assert!(trigram_sql.contains("trigram case_sensitive 0"));

    Ok(())
}

#[test]
fn deterministic_ids_are_stable() {
    let first_node_id = node_id(1, "rust", "file:///demo/src/lib.rs#function:caller:1:0");
    let second_node_id = node_id(1, "rust", "file:///demo/src/lib.rs#function:caller:1:0");
    assert_eq!(first_node_id, second_node_id);

    let first_edge_id = edge_id(1, &first_node_id, "callee", "calls", None);
    let second_edge_id = edge_id(1, &first_node_id, "callee", "calls", None);
    assert_eq!(first_edge_id, second_edge_id);
}

#[tokio::test]
async fn demo_seed_inserts_core_graph_rows() -> Result<(), Box<dyn Error>> {
    let store = migrated_store().await?;

    store.demo_seed("file:///demo").await?;

    assert_eq!(
        store.stats().await?,
        GraphStoreStats {
            workspaces: 1,
            extraction_runs: 1,
            files: 1,
            nodes: 2,
            edges: 1,
            occurrences: 1,
            edge_evidence: 1,
        }
    );

    Ok(())
}

#[tokio::test]
async fn upserts_do_not_duplicate_canonical_rows() -> Result<(), Box<dyn Error>> {
    let store = migrated_store().await?;

    let first = store.demo_seed("file:///demo").await?;
    let second = store.demo_seed("file:///demo").await?;

    assert_eq!(first.workspace_id, second.workspace_id);
    assert_eq!(first.file_id, second.file_id);
    assert_eq!(first.caller_node_id, second.caller_node_id);
    assert_eq!(first.callee_node_id, second.callee_node_id);
    assert_eq!(first.edge_id, second.edge_id);
    assert_eq!(
        store.stats().await?,
        GraphStoreStats {
            workspaces: 1,
            extraction_runs: 2,
            files: 1,
            nodes: 2,
            edges: 1,
            occurrences: 2,
            edge_evidence: 2,
        }
    );

    Ok(())
}

#[tokio::test]
async fn foreign_keys_reject_invalid_edge_references() -> Result<(), Box<dyn Error>> {
    let store = migrated_store().await?;
    let workspace_id = store.create_workspace("file:///demo", "rust").await?;

    let error = store
        .upsert_edge(EdgeInput {
            workspace_id,
            src_node_id: "missing-src",
            dst_node_id: "missing-dst",
            relation: "calls",
            context: None,
            confidence: "EXTRACTED",
            confidence_score: 1.0,
            weight: 1.0,
            properties_json: json!({}),
            run_id: None,
        })
        .await;

    assert!(error.is_err());

    Ok(())
}

#[tokio::test]
async fn route_status_tracks_start_complete_and_fail() -> Result<(), Box<dyn Error>> {
    let (store, path) = migrated_store_with_path().await?;
    let workspace_id = store.create_workspace("file:///demo", "rust").await?;
    let first_run_id = store
        .start_run(workspace_id, PROVIDER, Some("fixture"), None)
        .await?;

    let first_status_id = store
        .start_route_status(RouteStatusStartInput {
            workspace_id,
            route: ROUTE_REFERENCES,
            scope: SCOPE_WORKSPACE,
            scope_key: "file:///demo",
            file_id: None,
            provider: PROVIDER,
            provider_version: Some("fixture"),
            content_hash: Some("hash-a"),
            run_id: first_run_id,
            diagnostics_json: json!({}),
        })
        .await?;
    store
        .complete_route_status(RouteStatusCompleteInput {
            workspace_id,
            route: ROUTE_REFERENCES,
            scope: SCOPE_WORKSPACE,
            scope_key: "file:///demo",
            provider: PROVIDER,
            provider_version: Some("fixture"),
            content_hash: Some("hash-a"),
            run_id: first_run_id,
            diagnostics_json: json!({ "references": 1 }),
        })
        .await?;

    let second_run_id = store
        .start_run(workspace_id, PROVIDER, Some("fixture"), None)
        .await?;
    let second_status_id = store
        .start_route_status(RouteStatusStartInput {
            workspace_id,
            route: ROUTE_REFERENCES,
            scope: SCOPE_WORKSPACE,
            scope_key: "file:///demo",
            file_id: None,
            provider: PROVIDER,
            provider_version: Some("fixture"),
            content_hash: Some("hash-b"),
            run_id: second_run_id,
            diagnostics_json: json!({}),
        })
        .await?;
    store
        .fail_route_status(RouteStatusFailInput {
            workspace_id,
            route: ROUTE_REFERENCES,
            scope: SCOPE_WORKSPACE,
            scope_key: "file:///demo",
            provider: PROVIDER,
            run_id: second_run_id,
            diagnostics_json: json!({ "error": "fixture failure" }),
        })
        .await?;

    assert_eq!(first_status_id, second_status_id);

    let pool = sqlite_pool(&path).await?;
    let row: (String, i64, i64, String, String) = sqlx::query_as(
        r#"
        SELECT
          last_status,
          last_started_run_id,
          last_complete_run_id,
          content_hash,
          diagnostics_json
        FROM extraction_route_status
        WHERE id = ?
        "#,
    )
    .bind(first_status_id)
    .fetch_one(&pool)
    .await?;

    assert_eq!(row.0, "failed");
    assert_eq!(row.1, second_run_id);
    assert_eq!(row.2, first_run_id);
    assert_eq!(row.3, "hash-b");
    assert_eq!(row.4, "{\"error\":\"fixture failure\"}");

    Ok(())
}

#[tokio::test]
async fn route_observations_are_unique_per_route_run_and_entity() -> Result<(), Box<dyn Error>> {
    let (store, path) = migrated_store_with_path().await?;
    let workspace_id = store.create_workspace("file:///demo", "rust").await?;
    let run_id = store.start_run(workspace_id, PROVIDER, None, None).await?;
    let file_id = insert_file(&store, workspace_id, run_id, "src/lib.rs").await?;

    for _ in 0..2 {
        store
            .record_route_observation(RouteObservationInput {
                workspace_id,
                run_id,
                route: ROUTE_DOCUMENT_SYMBOLS,
                scope: SCOPE_FILE,
                scope_key: "file:///demo/src/lib.rs",
                provider: PROVIDER,
                entity_kind: "node",
                entity_id: "node-a",
                source_file_id: Some(file_id),
                properties_json: json!({ "observed": true }),
            })
            .await?;
    }

    let pool = sqlite_pool(&path).await?;
    let observation_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM route_observations")
        .fetch_one(&pool)
        .await?;
    assert_eq!(observation_count, 1);

    Ok(())
}

#[tokio::test]
async fn document_symbol_route_closes_stale_nodes() -> Result<(), Box<dyn Error>> {
    let (store, path) = migrated_store_with_path().await?;
    let workspace_id = store.create_workspace("file:///demo", "rust").await?;
    let first_run_id = store.start_run(workspace_id, PROVIDER, None, None).await?;
    let file_id = insert_file(&store, workspace_id, first_run_id, "src/lib.rs").await?;
    let scope_key = "file:///demo/src/lib.rs";
    let current_node_id =
        insert_node(&store, workspace_id, first_run_id, file_id, "current").await?;
    let stale_node_id = insert_node(&store, workspace_id, first_run_id, file_id, "stale").await?;

    complete_file_route(&store, workspace_id, first_run_id, file_id, scope_key).await?;
    record_node_observation(
        &store,
        workspace_id,
        first_run_id,
        scope_key,
        file_id,
        &current_node_id,
    )
    .await?;
    record_node_observation(
        &store,
        workspace_id,
        first_run_id,
        scope_key,
        file_id,
        &stale_node_id,
    )
    .await?;

    let second_run_id = store.start_run(workspace_id, PROVIDER, None, None).await?;
    insert_node(&store, workspace_id, second_run_id, file_id, "current").await?;
    complete_file_route(&store, workspace_id, second_run_id, file_id, scope_key).await?;
    record_node_observation(
        &store,
        workspace_id,
        second_run_id,
        scope_key,
        file_id,
        &current_node_id,
    )
    .await?;

    let closed = store
        .close_stale_nodes_for_route(CloseStaleRouteInput {
            workspace_id,
            run_id: second_run_id,
            route: ROUTE_DOCUMENT_SYMBOLS,
            scope: SCOPE_FILE,
            scope_key,
            provider: PROVIDER,
        })
        .await?;

    assert_eq!(closed, 1);
    let pool = sqlite_pool(&path).await?;
    assert_eq!(node_valid_to(&pool, &current_node_id).await?, None);
    assert_eq!(
        node_valid_to(&pool, &stale_node_id).await?,
        Some(second_run_id)
    );

    Ok(())
}

#[tokio::test]
async fn document_symbol_route_closes_stale_contains_edges() -> Result<(), Box<dyn Error>> {
    let (store, path) = migrated_store_with_path().await?;
    let workspace_id = store.create_workspace("file:///demo", "rust").await?;
    let first_run_id = store.start_run(workspace_id, PROVIDER, None, None).await?;
    let file_id = insert_file(&store, workspace_id, first_run_id, "src/lib.rs").await?;
    let scope_key = "file:///demo/src/lib.rs";
    let parent_id = insert_node(&store, workspace_id, first_run_id, file_id, "parent").await?;
    let current_child_id =
        insert_node(&store, workspace_id, first_run_id, file_id, "current-child").await?;
    let stale_child_id =
        insert_node(&store, workspace_id, first_run_id, file_id, "stale-child").await?;
    let current_edge_id = insert_edge(
        &store,
        workspace_id,
        first_run_id,
        &parent_id,
        &current_child_id,
        "contains",
    )
    .await?;
    let stale_edge_id = insert_edge(
        &store,
        workspace_id,
        first_run_id,
        &parent_id,
        &stale_child_id,
        "contains",
    )
    .await?;

    complete_file_route(&store, workspace_id, first_run_id, file_id, scope_key).await?;
    record_edge_observation(
        &store,
        workspace_id,
        first_run_id,
        scope_key,
        file_id,
        &current_edge_id,
    )
    .await?;
    record_edge_observation(
        &store,
        workspace_id,
        first_run_id,
        scope_key,
        file_id,
        &stale_edge_id,
    )
    .await?;

    let second_run_id = store.start_run(workspace_id, PROVIDER, None, None).await?;
    insert_edge(
        &store,
        workspace_id,
        second_run_id,
        &parent_id,
        &current_child_id,
        "contains",
    )
    .await?;
    complete_file_route(&store, workspace_id, second_run_id, file_id, scope_key).await?;
    record_edge_observation(
        &store,
        workspace_id,
        second_run_id,
        scope_key,
        file_id,
        &current_edge_id,
    )
    .await?;

    let closed = store
        .close_stale_edges_for_route(CloseStaleRouteInput {
            workspace_id,
            run_id: second_run_id,
            route: ROUTE_DOCUMENT_SYMBOLS,
            scope: SCOPE_FILE,
            scope_key,
            provider: PROVIDER,
        })
        .await?;

    assert_eq!(closed, 1);
    let pool = sqlite_pool(&path).await?;
    assert_eq!(edge_valid_to(&pool, &current_edge_id).await?, None);
    assert_eq!(
        edge_valid_to(&pool, &stale_edge_id).await?,
        Some(second_run_id)
    );

    Ok(())
}

#[tokio::test]
async fn reference_route_closes_stale_edges() -> Result<(), Box<dyn Error>> {
    let (store, path) = migrated_store_with_path().await?;
    let workspace_id = store.create_workspace("file:///demo", "rust").await?;
    let first_run_id = store.start_run(workspace_id, PROVIDER, None, None).await?;
    let file_id = insert_file(&store, workspace_id, first_run_id, "src/lib.rs").await?;
    let scope_key = "file:///demo";
    let source_id = insert_node(&store, workspace_id, first_run_id, file_id, "source").await?;
    let current_target_id = insert_node(
        &store,
        workspace_id,
        first_run_id,
        file_id,
        "current-target",
    )
    .await?;
    let stale_target_id =
        insert_node(&store, workspace_id, first_run_id, file_id, "stale-target").await?;
    let current_edge_id = insert_edge(
        &store,
        workspace_id,
        first_run_id,
        &source_id,
        &current_target_id,
        "references",
    )
    .await?;
    let stale_edge_id = insert_edge(
        &store,
        workspace_id,
        first_run_id,
        &source_id,
        &stale_target_id,
        "references",
    )
    .await?;

    complete_workspace_route(&store, workspace_id, first_run_id, scope_key).await?;
    record_reference_observation(
        &store,
        workspace_id,
        first_run_id,
        scope_key,
        file_id,
        &current_edge_id,
    )
    .await?;
    record_reference_observation(
        &store,
        workspace_id,
        first_run_id,
        scope_key,
        file_id,
        &stale_edge_id,
    )
    .await?;

    let second_run_id = store.start_run(workspace_id, PROVIDER, None, None).await?;
    insert_edge(
        &store,
        workspace_id,
        second_run_id,
        &source_id,
        &current_target_id,
        "references",
    )
    .await?;
    complete_workspace_route(&store, workspace_id, second_run_id, scope_key).await?;
    record_reference_observation(
        &store,
        workspace_id,
        second_run_id,
        scope_key,
        file_id,
        &current_edge_id,
    )
    .await?;

    let closed = store
        .close_stale_edges_for_route(CloseStaleRouteInput {
            workspace_id,
            run_id: second_run_id,
            route: ROUTE_REFERENCES,
            scope: SCOPE_WORKSPACE,
            scope_key,
            provider: PROVIDER,
        })
        .await?;

    assert_eq!(closed, 1);
    let pool = sqlite_pool(&path).await?;
    assert_eq!(edge_valid_to(&pool, &current_edge_id).await?, None);
    assert_eq!(
        edge_valid_to(&pool, &stale_edge_id).await?,
        Some(second_run_id)
    );

    Ok(())
}

#[tokio::test]
async fn failed_route_does_not_close_stale_edges() -> Result<(), Box<dyn Error>> {
    let (store, path) = migrated_store_with_path().await?;
    let workspace_id = store.create_workspace("file:///demo", "rust").await?;
    let first_run_id = store.start_run(workspace_id, PROVIDER, None, None).await?;
    let file_id = insert_file(&store, workspace_id, first_run_id, "src/lib.rs").await?;
    let scope_key = "file:///demo";
    let source_id = insert_node(&store, workspace_id, first_run_id, file_id, "source").await?;
    let target_id = insert_node(&store, workspace_id, first_run_id, file_id, "target").await?;
    let edge_id = insert_edge(
        &store,
        workspace_id,
        first_run_id,
        &source_id,
        &target_id,
        "references",
    )
    .await?;

    complete_workspace_route(&store, workspace_id, first_run_id, scope_key).await?;
    record_reference_observation(
        &store,
        workspace_id,
        first_run_id,
        scope_key,
        file_id,
        &edge_id,
    )
    .await?;

    let second_run_id = store.start_run(workspace_id, PROVIDER, None, None).await?;
    store
        .start_route_status(RouteStatusStartInput {
            workspace_id,
            route: ROUTE_REFERENCES,
            scope: SCOPE_WORKSPACE,
            scope_key,
            file_id: None,
            provider: PROVIDER,
            provider_version: None,
            content_hash: None,
            run_id: second_run_id,
            diagnostics_json: json!({}),
        })
        .await?;
    store
        .fail_route_status(RouteStatusFailInput {
            workspace_id,
            route: ROUTE_REFERENCES,
            scope: SCOPE_WORKSPACE,
            scope_key,
            provider: PROVIDER,
            run_id: second_run_id,
            diagnostics_json: json!({ "error": "fixture failure" }),
        })
        .await?;

    let closed = store
        .close_stale_edges_for_route(CloseStaleRouteInput {
            workspace_id,
            run_id: second_run_id,
            route: ROUTE_REFERENCES,
            scope: SCOPE_WORKSPACE,
            scope_key,
            provider: PROVIDER,
        })
        .await?;

    assert_eq!(closed, 0);
    let pool = sqlite_pool(&path).await?;
    assert_eq!(edge_valid_to(&pool, &edge_id).await?, None);

    Ok(())
}

#[tokio::test]
async fn call_route_closes_stale_edges() -> Result<(), Box<dyn Error>> {
    let (store, path) = migrated_store_with_path().await?;
    let workspace_id = store.create_workspace("file:///demo", "rust").await?;
    let first_run_id = store.start_run(workspace_id, PROVIDER, None, None).await?;
    let file_id = insert_file(&store, workspace_id, first_run_id, "src/lib.rs").await?;
    let scope_key = "file:///demo";
    let caller_id = insert_node(&store, workspace_id, first_run_id, file_id, "caller").await?;
    let current_callee_id = insert_node(
        &store,
        workspace_id,
        first_run_id,
        file_id,
        "current-callee",
    )
    .await?;
    let stale_callee_id =
        insert_node(&store, workspace_id, first_run_id, file_id, "stale-callee").await?;
    let current_edge_id = insert_edge(
        &store,
        workspace_id,
        first_run_id,
        &caller_id,
        &current_callee_id,
        "calls",
    )
    .await?;
    let stale_edge_id = insert_edge(
        &store,
        workspace_id,
        first_run_id,
        &caller_id,
        &stale_callee_id,
        "calls",
    )
    .await?;

    complete_call_route(&store, workspace_id, first_run_id, scope_key).await?;
    record_call_observation(
        &store,
        workspace_id,
        first_run_id,
        scope_key,
        file_id,
        &current_edge_id,
    )
    .await?;
    record_call_observation(
        &store,
        workspace_id,
        first_run_id,
        scope_key,
        file_id,
        &stale_edge_id,
    )
    .await?;

    let second_run_id = store.start_run(workspace_id, PROVIDER, None, None).await?;
    insert_edge(
        &store,
        workspace_id,
        second_run_id,
        &caller_id,
        &current_callee_id,
        "calls",
    )
    .await?;
    complete_call_route(&store, workspace_id, second_run_id, scope_key).await?;
    record_call_observation(
        &store,
        workspace_id,
        second_run_id,
        scope_key,
        file_id,
        &current_edge_id,
    )
    .await?;

    let closed = store
        .close_stale_edges_for_route(CloseStaleRouteInput {
            workspace_id,
            run_id: second_run_id,
            route: ROUTE_CALLS,
            scope: SCOPE_WORKSPACE,
            scope_key,
            provider: PROVIDER,
        })
        .await?;

    assert_eq!(closed, 1);
    let pool = sqlite_pool(&path).await?;
    assert_eq!(edge_valid_to(&pool, &current_edge_id).await?, None);
    assert_eq!(
        edge_valid_to(&pool, &stale_edge_id).await?,
        Some(second_run_id)
    );

    Ok(())
}

#[tokio::test]
async fn upsert_reopens_reobserved_stale_node() -> Result<(), Box<dyn Error>> {
    let (store, path) = migrated_store_with_path().await?;
    let workspace_id = store.create_workspace("file:///demo", "rust").await?;
    let first_run_id = store.start_run(workspace_id, PROVIDER, None, None).await?;
    let file_id = insert_file(&store, workspace_id, first_run_id, "src/lib.rs").await?;
    let scope_key = "file:///demo/src/lib.rs";
    let node_id = insert_node(&store, workspace_id, first_run_id, file_id, "reopened").await?;

    complete_file_route(&store, workspace_id, first_run_id, file_id, scope_key).await?;
    record_node_observation(
        &store,
        workspace_id,
        first_run_id,
        scope_key,
        file_id,
        &node_id,
    )
    .await?;

    let second_run_id = store.start_run(workspace_id, PROVIDER, None, None).await?;
    complete_file_route(&store, workspace_id, second_run_id, file_id, scope_key).await?;
    let closed = store
        .close_stale_nodes_for_route(CloseStaleRouteInput {
            workspace_id,
            run_id: second_run_id,
            route: ROUTE_DOCUMENT_SYMBOLS,
            scope: SCOPE_FILE,
            scope_key,
            provider: PROVIDER,
        })
        .await?;
    assert_eq!(closed, 1);

    let third_run_id = store.start_run(workspace_id, PROVIDER, None, None).await?;
    insert_node(&store, workspace_id, third_run_id, file_id, "reopened").await?;
    complete_file_route(&store, workspace_id, third_run_id, file_id, scope_key).await?;
    record_node_observation(
        &store,
        workspace_id,
        third_run_id,
        scope_key,
        file_id,
        &node_id,
    )
    .await?;

    let pool = sqlite_pool(&path).await?;
    assert_eq!(node_valid_to(&pool, &node_id).await?, None);

    Ok(())
}

#[tokio::test]
async fn upsert_reopens_reobserved_stale_call_edge() -> Result<(), Box<dyn Error>> {
    let (store, path) = migrated_store_with_path().await?;
    let workspace_id = store.create_workspace("file:///demo", "rust").await?;
    let first_run_id = store.start_run(workspace_id, PROVIDER, None, None).await?;
    let file_id = insert_file(&store, workspace_id, first_run_id, "src/lib.rs").await?;
    let scope_key = "file:///demo";
    let caller_id = insert_node(&store, workspace_id, first_run_id, file_id, "caller").await?;
    let callee_id = insert_node(&store, workspace_id, first_run_id, file_id, "callee").await?;
    let edge_id = insert_edge(
        &store,
        workspace_id,
        first_run_id,
        &caller_id,
        &callee_id,
        "calls",
    )
    .await?;

    complete_call_route(&store, workspace_id, first_run_id, scope_key).await?;
    record_call_observation(
        &store,
        workspace_id,
        first_run_id,
        scope_key,
        file_id,
        &edge_id,
    )
    .await?;

    let second_run_id = store.start_run(workspace_id, PROVIDER, None, None).await?;
    complete_call_route(&store, workspace_id, second_run_id, scope_key).await?;
    let closed = store
        .close_stale_edges_for_route(CloseStaleRouteInput {
            workspace_id,
            run_id: second_run_id,
            route: ROUTE_CALLS,
            scope: SCOPE_WORKSPACE,
            scope_key,
            provider: PROVIDER,
        })
        .await?;
    assert_eq!(closed, 1);

    let third_run_id = store.start_run(workspace_id, PROVIDER, None, None).await?;
    insert_edge(
        &store,
        workspace_id,
        third_run_id,
        &caller_id,
        &callee_id,
        "calls",
    )
    .await?;
    complete_call_route(&store, workspace_id, third_run_id, scope_key).await?;
    record_call_observation(
        &store,
        workspace_id,
        third_run_id,
        scope_key,
        file_id,
        &edge_id,
    )
    .await?;

    let pool = sqlite_pool(&path).await?;
    assert_eq!(edge_valid_to(&pool, &edge_id).await?, None);

    Ok(())
}

#[tokio::test]
async fn upsert_edge_updates_call_weight() -> Result<(), Box<dyn Error>> {
    let (store, path) = migrated_store_with_path().await?;
    let workspace_id = store.create_workspace("file:///demo", "rust").await?;
    let first_run_id = store.start_run(workspace_id, PROVIDER, None, None).await?;
    let file_id = insert_file(&store, workspace_id, first_run_id, "src/lib.rs").await?;
    let caller_id = insert_node(&store, workspace_id, first_run_id, file_id, "caller").await?;
    let callee_id = insert_node(&store, workspace_id, first_run_id, file_id, "callee").await?;
    let edge_id = store
        .upsert_edge(EdgeInput {
            workspace_id,
            src_node_id: &caller_id,
            dst_node_id: &callee_id,
            relation: "calls",
            context: Some("direct"),
            confidence: "EXTRACTED",
            confidence_score: 1.0,
            weight: 2.0,
            properties_json: json!({}),
            run_id: Some(first_run_id),
        })
        .await?;

    let second_run_id = store.start_run(workspace_id, PROVIDER, None, None).await?;
    store
        .upsert_edge(EdgeInput {
            workspace_id,
            src_node_id: &caller_id,
            dst_node_id: &callee_id,
            relation: "calls",
            context: Some("direct"),
            confidence: "EXTRACTED",
            confidence_score: 1.0,
            weight: 1.0,
            properties_json: json!({}),
            run_id: Some(second_run_id),
        })
        .await?;

    let pool = sqlite_pool(&path).await?;
    assert_eq!(edge_weight(&pool, &edge_id).await?, 1.0);

    Ok(())
}

async fn sqlite_pool(path: &Path) -> Result<SqlitePool, Box<dyn Error>> {
    Ok(SqlitePool::connect(&format!("sqlite://{}", path.display())).await?)
}

async fn insert_file(
    store: &TestStore,
    workspace_id: i64,
    run_id: i64,
    path: &str,
) -> Result<i64, Box<dyn Error>> {
    let uri = format!("file:///demo/{path}");
    Ok(store
        .upsert_file(FileInput {
            workspace_id,
            uri: &uri,
            path,
            language: "rust",
            content_hash: Some("fixture-hash"),
            last_seen_run_id: Some(run_id),
            properties_json: json!({}),
        })
        .await?)
}

async fn insert_node(
    store: &TestStore,
    workspace_id: i64,
    run_id: i64,
    file_id: i64,
    name: &str,
) -> Result<String, Box<dyn Error>> {
    let symbol_key = format!("file:///demo/src/lib.rs#function:{name}");
    Ok(store
        .upsert_node(NodeInput {
            workspace_id,
            language: "rust",
            kind: "function",
            name,
            qualified_name: Some(name),
            display_name: Some(name),
            symbol_key: &symbol_key,
            file_id: Some(file_id),
            range: Some(range(1, 0, 3, 1)),
            selection_range: Some(range(1, 3, 1, 3 + i64::try_from(name.len())?)),
            container_node_id: None,
            properties_json: json!({}),
            run_id: Some(run_id),
        })
        .await?)
}

async fn insert_edge(
    store: &TestStore,
    workspace_id: i64,
    run_id: i64,
    src_node_id: &str,
    dst_node_id: &str,
    relation: &str,
) -> Result<String, Box<dyn Error>> {
    Ok(store
        .upsert_edge(EdgeInput {
            workspace_id,
            src_node_id,
            dst_node_id,
            relation,
            context: None,
            confidence: "EXTRACTED",
            confidence_score: 1.0,
            weight: 1.0,
            properties_json: json!({}),
            run_id: Some(run_id),
        })
        .await?)
}

async fn complete_file_route(
    store: &TestStore,
    workspace_id: i64,
    run_id: i64,
    file_id: i64,
    scope_key: &str,
) -> Result<(), Box<dyn Error>> {
    store
        .start_route_status(RouteStatusStartInput {
            workspace_id,
            route: ROUTE_DOCUMENT_SYMBOLS,
            scope: SCOPE_FILE,
            scope_key,
            file_id: Some(file_id),
            provider: PROVIDER,
            provider_version: None,
            content_hash: Some("fixture-hash"),
            run_id,
            diagnostics_json: json!({}),
        })
        .await?;
    store
        .complete_route_status(RouteStatusCompleteInput {
            workspace_id,
            route: ROUTE_DOCUMENT_SYMBOLS,
            scope: SCOPE_FILE,
            scope_key,
            provider: PROVIDER,
            provider_version: None,
            content_hash: Some("fixture-hash"),
            run_id,
            diagnostics_json: json!({}),
        })
        .await?;
    Ok(())
}

async fn complete_workspace_route(
    store: &TestStore,
    workspace_id: i64,
    run_id: i64,
    scope_key: &str,
) -> Result<(), Box<dyn Error>> {
    store
        .start_route_status(RouteStatusStartInput {
            workspace_id,
            route: ROUTE_REFERENCES,
            scope: SCOPE_WORKSPACE,
            scope_key,
            file_id: None,
            provider: PROVIDER,
            provider_version: None,
            content_hash: Some("fixture-hash"),
            run_id,
            diagnostics_json: json!({}),
        })
        .await?;
    store
        .complete_route_status(RouteStatusCompleteInput {
            workspace_id,
            route: ROUTE_REFERENCES,
            scope: SCOPE_WORKSPACE,
            scope_key,
            provider: PROVIDER,
            provider_version: None,
            content_hash: Some("fixture-hash"),
            run_id,
            diagnostics_json: json!({}),
        })
        .await?;
    Ok(())
}

async fn complete_call_route(
    store: &TestStore,
    workspace_id: i64,
    run_id: i64,
    scope_key: &str,
) -> Result<(), Box<dyn Error>> {
    store
        .start_route_status(RouteStatusStartInput {
            workspace_id,
            route: ROUTE_CALLS,
            scope: SCOPE_WORKSPACE,
            scope_key,
            file_id: None,
            provider: PROVIDER,
            provider_version: None,
            content_hash: Some("fixture-hash"),
            run_id,
            diagnostics_json: json!({}),
        })
        .await?;
    store
        .complete_route_status(RouteStatusCompleteInput {
            workspace_id,
            route: ROUTE_CALLS,
            scope: SCOPE_WORKSPACE,
            scope_key,
            provider: PROVIDER,
            provider_version: None,
            content_hash: Some("fixture-hash"),
            run_id,
            diagnostics_json: json!({}),
        })
        .await?;
    Ok(())
}

async fn record_node_observation(
    store: &TestStore,
    workspace_id: i64,
    run_id: i64,
    scope_key: &str,
    file_id: i64,
    node_id: &str,
) -> Result<(), Box<dyn Error>> {
    store
        .record_route_observation(RouteObservationInput {
            workspace_id,
            run_id,
            route: ROUTE_DOCUMENT_SYMBOLS,
            scope: SCOPE_FILE,
            scope_key,
            provider: PROVIDER,
            entity_kind: "node",
            entity_id: node_id,
            source_file_id: Some(file_id),
            properties_json: json!({}),
        })
        .await?;
    Ok(())
}

async fn record_edge_observation(
    store: &TestStore,
    workspace_id: i64,
    run_id: i64,
    scope_key: &str,
    file_id: i64,
    edge_id: &str,
) -> Result<(), Box<dyn Error>> {
    store
        .record_route_observation(RouteObservationInput {
            workspace_id,
            run_id,
            route: ROUTE_DOCUMENT_SYMBOLS,
            scope: SCOPE_FILE,
            scope_key,
            provider: PROVIDER,
            entity_kind: "edge",
            entity_id: edge_id,
            source_file_id: Some(file_id),
            properties_json: json!({}),
        })
        .await?;
    Ok(())
}

async fn record_reference_observation(
    store: &TestStore,
    workspace_id: i64,
    run_id: i64,
    scope_key: &str,
    file_id: i64,
    edge_id: &str,
) -> Result<(), Box<dyn Error>> {
    store
        .record_route_observation(RouteObservationInput {
            workspace_id,
            run_id,
            route: ROUTE_REFERENCES,
            scope: SCOPE_WORKSPACE,
            scope_key,
            provider: PROVIDER,
            entity_kind: "edge",
            entity_id: edge_id,
            source_file_id: Some(file_id),
            properties_json: json!({}),
        })
        .await?;
    Ok(())
}

async fn record_call_observation(
    store: &TestStore,
    workspace_id: i64,
    run_id: i64,
    scope_key: &str,
    file_id: i64,
    edge_id: &str,
) -> Result<(), Box<dyn Error>> {
    store
        .record_route_observation(RouteObservationInput {
            workspace_id,
            run_id,
            route: ROUTE_CALLS,
            scope: SCOPE_WORKSPACE,
            scope_key,
            provider: PROVIDER,
            entity_kind: "edge",
            entity_id: edge_id,
            source_file_id: Some(file_id),
            properties_json: json!({}),
        })
        .await?;
    Ok(())
}

async fn node_valid_to(pool: &SqlitePool, node_id: &str) -> Result<Option<i64>, Box<dyn Error>> {
    Ok(
        sqlx::query_scalar("SELECT valid_to_run_id FROM nodes WHERE id = ?")
            .bind(node_id)
            .fetch_one(pool)
            .await?,
    )
}

async fn edge_valid_to(pool: &SqlitePool, edge_id: &str) -> Result<Option<i64>, Box<dyn Error>> {
    Ok(
        sqlx::query_scalar("SELECT valid_to_run_id FROM edges WHERE id = ?")
            .bind(edge_id)
            .fetch_one(pool)
            .await?,
    )
}

async fn edge_weight(pool: &SqlitePool, edge_id: &str) -> Result<f64, Box<dyn Error>> {
    Ok(sqlx::query_scalar("SELECT weight FROM edges WHERE id = ?")
        .bind(edge_id)
        .fetch_one(pool)
        .await?)
}

fn range(start_line: i64, start_col: i64, end_line: i64, end_col: i64) -> TextRange {
    TextRange {
        start_line,
        start_col,
        end_line,
        end_col,
    }
}
