use crate::{
    CloseStaleFileInput, CloseStaleRouteInput, Config,
    DocumentSymbolWriteBatchCloseStaleRouteInput, DocumentSymbolWriteBatchEdgeEvidenceInput,
    DocumentSymbolWriteBatchFileInput, DocumentSymbolWriteBatchInput,
    DocumentSymbolWriteBatchNodeInput, DocumentSymbolWriteBatchObservationInput,
    DocumentSymbolWriteBatchOccurrenceInput, DocumentSymbolWriteBatchRouteStatusCompleteInput,
    DocumentSymbolWriteBatchRouteStatusStartInput, EdgeEvidenceInput, EdgeInput, FileInput,
    NodeInput, RouteObservationInput, RouteStatusCompleteInput, RouteStatusStartInput,
    RouteWriteBatchEdgeEvidenceInput, RouteWriteBatchEdgeInput, RouteWriteBatchInput,
    RouteWriteBatchObservationInput, RouteWriteBatchOccurrenceInput, TextRange, WriteManager,
    edge_id, node_id,
};
use serde_json::json;
use sqlx::SqlitePool;
use std::{
    env,
    error::Error,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn deterministic_ids_are_stable() {
    let first_node_id = node_id(1, "rust", "file:///demo/src/lib.rs#function:caller:1:0");
    let second_node_id = node_id(1, "rust", "file:///demo/src/lib.rs#function:caller:1:0");
    assert_eq!(first_node_id, second_node_id);
    assert_eq!(
        first_node_id,
        "028199ced09ed29adb1aaf9521f63e9d90ef333aba2066f90c51ce4be1739b9c"
    );

    let first_edge_id = edge_id(1, &first_node_id, "callee", "calls", None);
    let second_edge_id = edge_id(1, &first_node_id, "callee", "calls", None);
    assert_eq!(first_edge_id, second_edge_id);
    assert_eq!(
        first_edge_id,
        "3853fd7d3afaa05a34a9142501247d7ec4aabbd056df2cdc842de489766c5193"
    );
}

#[test]
fn default_write_config_matches_plan_defaults() {
    let config = Config::default();

    assert_eq!(config.queue_capacity(), 4096);
    assert_eq!(config.max_rows_per_commit(), 1000);
    assert_eq!(config.max_millis_per_commit(), 250);
    assert_eq!(config.busy_timeout_ms(), 5000);
}

#[tokio::test]
async fn shutdown_waits_for_sqlite_pool_cleanup() -> Result<(), Box<dyn Error>> {
    let path = temp_db_path()?;
    let writer = WriteManager::start(&path).await?;

    writer.migrate().await?;
    writer
        .create_workspace("file:///tmp/db-manager-shutdown", "rust")
        .await?;
    writer.shutdown().await?;

    assert!(!sidecar_path(&path, "shm").exists());
    assert!(!sidecar_path(&path, "wal").exists());

    Ok(())
}

#[tokio::test]
async fn route_write_batch_writes_related_route_rows() -> Result<(), Box<dyn Error>> {
    let path = temp_db_path()?;
    let writer = WriteManager::start(&path).await?;
    writer.migrate().await?;

    let workspace_uri = "file:///tmp/db-manager-route-write-batch";
    let workspace_id = writer.create_workspace(workspace_uri, "rust").await?;
    let run_id = writer
        .start_run(workspace_id, "rust-analyzer", Some("test"), None)
        .await?;
    let file_uri = "file:///tmp/db-manager-route-write-batch/src/lib.rs";
    let file_id = writer
        .upsert_file(FileInput {
            workspace_id,
            uri: file_uri,
            path: "src/lib.rs",
            language: "rust",
            content_hash: None,
            last_seen_run_id: Some(run_id),
            properties_json: json!({}),
        })
        .await?;
    let source_symbol_key = format!("{file_uri}#function:source:1:0");
    let target_symbol_key = format!("{file_uri}#function:target:5:0");
    let source_node_id = writer
        .upsert_node(NodeInput {
            workspace_id,
            language: "rust",
            kind: "function",
            name: "source",
            qualified_name: Some("source"),
            display_name: Some("source"),
            symbol_key: &source_symbol_key,
            file_id: Some(file_id),
            range: None,
            selection_range: None,
            container_node_id: None,
            properties_json: json!({}),
            run_id: Some(run_id),
        })
        .await?;
    let target_node_id = writer
        .upsert_node(NodeInput {
            workspace_id,
            language: "rust",
            kind: "function",
            name: "target",
            qualified_name: Some("target"),
            display_name: Some("target"),
            symbol_key: &target_symbol_key,
            file_id: Some(file_id),
            range: None,
            selection_range: None,
            container_node_id: None,
            properties_json: json!({}),
            run_id: Some(run_id),
        })
        .await?;
    let edge_id = edge_id(
        workspace_id,
        &source_node_id,
        &target_node_id,
        "references",
        Some("symbol"),
    );
    let range = TextRange {
        start_line: 3,
        start_col: 4,
        end_line: 3,
        end_col: 10,
    };

    writer
        .write_route_batch(RouteWriteBatchInput {
            edges: vec![RouteWriteBatchEdgeInput {
                workspace_id,
                src_node_id: source_node_id.clone(),
                dst_node_id: target_node_id.clone(),
                relation: "references".to_string(),
                context: Some("symbol".to_string()),
                confidence: "EXTRACTED".to_string(),
                confidence_score: 1.0,
                weight: 1.0,
                properties_json: json!({ "route": "rust.references" }),
                run_id: Some(run_id),
            }],
            occurrences: vec![RouteWriteBatchOccurrenceInput {
                node_id: target_node_id,
                run_id,
                file_id,
                role: "reference".to_string(),
                range,
                enclosing_node_id: Some(source_node_id),
                raw_json: Some(json!({ "kind": "occurrence" })),
            }],
            edge_evidence: vec![RouteWriteBatchEdgeEvidenceInput {
                edge_id: edge_id.clone(),
                run_id,
                provider: "rust-analyzer".to_string(),
                lsp_method: Some("textDocument/references".to_string()),
                file_id: Some(file_id),
                range: Some(range),
                raw_json: Some(json!({ "kind": "evidence" })),
            }],
            route_observations: vec![RouteWriteBatchObservationInput {
                workspace_id,
                run_id,
                route: "rust.references".to_string(),
                scope: "workspace".to_string(),
                scope_key: workspace_uri.to_string(),
                provider: "rust-analyzer".to_string(),
                entity_kind: "edge".to_string(),
                entity_id: edge_id.clone(),
                source_file_id: Some(file_id),
                properties_json: json!({ "source": "textDocument/references" }),
            }],
        })
        .await?;
    writer.finish_run(run_id, "complete").await?;
    writer.shutdown().await?;

    let pool = sqlite_pool(&path).await?;
    let edge_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM edges WHERE id = ?")
        .bind(&edge_id)
        .fetch_one(&pool)
        .await?;
    let occurrence_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM occurrences WHERE run_id = ?")
            .bind(run_id)
            .fetch_one(&pool)
            .await?;
    let evidence_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM edge_evidence WHERE edge_id = ?")
            .bind(&edge_id)
            .fetch_one(&pool)
            .await?;
    let observation_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM route_observations WHERE entity_id = ?")
            .bind(&edge_id)
            .fetch_one(&pool)
            .await?;

    assert_eq!(edge_count, 1);
    assert_eq!(occurrence_count, 1);
    assert_eq!(evidence_count, 1);
    assert_eq!(observation_count, 1);

    Ok(())
}

#[tokio::test]
async fn document_symbol_write_batch_writes_file_route_rows() -> Result<(), Box<dyn Error>> {
    let path = temp_db_path()?;
    let writer = WriteManager::start(&path).await?;
    writer.migrate().await?;

    let workspace_uri = "file:///tmp/db-manager-document-symbol-write-batch";
    let workspace_id = writer.create_workspace(workspace_uri, "rust").await?;
    let run_id = writer
        .start_run(workspace_id, "rust-analyzer", Some("test"), None)
        .await?;
    let file_uri = "file:///tmp/db-manager-document-symbol-write-batch/src/lib.rs";
    let file_symbol_key = format!("{file_uri}#file");
    let function_symbol_key = format!("{file_uri}#function:process:3:0");
    let file_node_id = node_id(workspace_id, "rust", &file_symbol_key);
    let function_node_id = node_id(workspace_id, "rust", &function_symbol_key);
    let contains_edge_id = edge_id(
        workspace_id,
        &file_node_id,
        &function_node_id,
        "contains",
        None,
    );
    let range = TextRange {
        start_line: 3,
        start_col: 0,
        end_line: 3,
        end_col: 7,
    };

    let summary = writer
        .write_document_symbol_batch(DocumentSymbolWriteBatchInput {
            files: vec![DocumentSymbolWriteBatchFileInput {
                workspace_id,
                uri: file_uri.to_string(),
                path: "src/lib.rs".to_string(),
                language: "rust".to_string(),
                content_hash: Some("hash".to_string()),
                last_seen_run_id: Some(run_id),
                properties_json: json!({ "provider": "rust-analyzer" }),
            }],
            route_status_starts: vec![DocumentSymbolWriteBatchRouteStatusStartInput {
                workspace_id,
                route: "rust.document_symbols".to_string(),
                scope: "file".to_string(),
                scope_key: file_uri.to_string(),
                file_uri: Some(file_uri.to_string()),
                provider: "rust-analyzer".to_string(),
                provider_version: Some("test".to_string()),
                content_hash: Some("hash".to_string()),
                run_id,
                diagnostics_json: json!({}),
            }],
            nodes: vec![
                DocumentSymbolWriteBatchNodeInput {
                    workspace_id,
                    language: "rust".to_string(),
                    kind: "file".to_string(),
                    name: "lib.rs".to_string(),
                    qualified_name: Some("src/lib.rs".to_string()),
                    display_name: Some("lib.rs".to_string()),
                    symbol_key: file_symbol_key,
                    file_uri: Some(file_uri.to_string()),
                    range: None,
                    selection_range: None,
                    container_node_id: None,
                    properties_json: json!({ "kind": "file" }),
                    run_id: Some(run_id),
                },
                DocumentSymbolWriteBatchNodeInput {
                    workspace_id,
                    language: "rust".to_string(),
                    kind: "function".to_string(),
                    name: "process".to_string(),
                    qualified_name: Some("process".to_string()),
                    display_name: Some("process".to_string()),
                    symbol_key: function_symbol_key,
                    file_uri: Some(file_uri.to_string()),
                    range: Some(range),
                    selection_range: Some(range),
                    container_node_id: Some(file_node_id.clone()),
                    properties_json: json!({ "kind": "function" }),
                    run_id: Some(run_id),
                },
            ],
            occurrences: vec![DocumentSymbolWriteBatchOccurrenceInput {
                node_id: function_node_id.clone(),
                run_id,
                file_uri: file_uri.to_string(),
                role: "definition".to_string(),
                range,
                enclosing_node_id: Some(file_node_id.clone()),
                raw_json: Some(json!({ "kind": "definition" })),
            }],
            edges: vec![RouteWriteBatchEdgeInput {
                workspace_id,
                src_node_id: file_node_id.clone(),
                dst_node_id: function_node_id.clone(),
                relation: "contains".to_string(),
                context: None,
                confidence: "EXTRACTED".to_string(),
                confidence_score: 1.0,
                weight: 1.0,
                properties_json: json!({ "source": "textDocument/documentSymbol" }),
                run_id: Some(run_id),
            }],
            edge_evidence: vec![DocumentSymbolWriteBatchEdgeEvidenceInput {
                edge_id: contains_edge_id.clone(),
                run_id,
                provider: "rust-analyzer".to_string(),
                lsp_method: Some("textDocument/documentSymbol".to_string()),
                file_uri: Some(file_uri.to_string()),
                range: Some(range),
                raw_json: Some(json!({ "kind": "contains" })),
            }],
            route_observations: vec![
                DocumentSymbolWriteBatchObservationInput {
                    workspace_id,
                    run_id,
                    route: "rust.document_symbols".to_string(),
                    scope: "file".to_string(),
                    scope_key: file_uri.to_string(),
                    provider: "rust-analyzer".to_string(),
                    entity_kind: "node".to_string(),
                    entity_id: function_node_id.clone(),
                    source_file_uri: Some(file_uri.to_string()),
                    properties_json: json!({ "source": "textDocument/documentSymbol" }),
                },
                DocumentSymbolWriteBatchObservationInput {
                    workspace_id,
                    run_id,
                    route: "rust.document_symbols".to_string(),
                    scope: "file".to_string(),
                    scope_key: file_uri.to_string(),
                    provider: "rust-analyzer".to_string(),
                    entity_kind: "edge".to_string(),
                    entity_id: contains_edge_id.clone(),
                    source_file_uri: Some(file_uri.to_string()),
                    properties_json: json!({ "source": "textDocument/documentSymbol" }),
                },
            ],
            route_status_completes: vec![DocumentSymbolWriteBatchRouteStatusCompleteInput {
                workspace_id,
                route: "rust.document_symbols".to_string(),
                scope: "file".to_string(),
                scope_key: file_uri.to_string(),
                provider: "rust-analyzer".to_string(),
                provider_version: Some("test".to_string()),
                content_hash: Some("hash".to_string()),
                run_id,
                diagnostics_json: json!({ "nodes": 2, "contains_edges": 1 }),
            }],
            close_stale_nodes: vec![DocumentSymbolWriteBatchCloseStaleRouteInput {
                workspace_id,
                run_id,
                route: "rust.document_symbols".to_string(),
                scope: "file".to_string(),
                scope_key: file_uri.to_string(),
                provider: "rust-analyzer".to_string(),
            }],
            close_stale_edges: vec![DocumentSymbolWriteBatchCloseStaleRouteInput {
                workspace_id,
                run_id,
                route: "rust.document_symbols".to_string(),
                scope: "file".to_string(),
                scope_key: file_uri.to_string(),
                provider: "rust-analyzer".to_string(),
            }],
        })
        .await?;
    writer.finish_run(run_id, "complete").await?;

    let second_run_id = writer
        .start_run(workspace_id, "rust-analyzer", Some("test"), None)
        .await?;
    let second_summary = writer
        .write_document_symbol_batch(DocumentSymbolWriteBatchInput {
            files: vec![DocumentSymbolWriteBatchFileInput {
                workspace_id,
                uri: file_uri.to_string(),
                path: "src/lib.rs".to_string(),
                language: "rust".to_string(),
                content_hash: Some("hash-2".to_string()),
                last_seen_run_id: Some(second_run_id),
                properties_json: json!({ "provider": "rust-analyzer" }),
            }],
            route_status_starts: vec![DocumentSymbolWriteBatchRouteStatusStartInput {
                workspace_id,
                route: "rust.document_symbols".to_string(),
                scope: "file".to_string(),
                scope_key: file_uri.to_string(),
                file_uri: Some(file_uri.to_string()),
                provider: "rust-analyzer".to_string(),
                provider_version: Some("test".to_string()),
                content_hash: Some("hash-2".to_string()),
                run_id: second_run_id,
                diagnostics_json: json!({}),
            }],
            nodes: vec![DocumentSymbolWriteBatchNodeInput {
                workspace_id,
                language: "rust".to_string(),
                kind: "file".to_string(),
                name: "lib.rs".to_string(),
                qualified_name: Some("src/lib.rs".to_string()),
                display_name: Some("lib.rs".to_string()),
                symbol_key: format!("{file_uri}#file"),
                file_uri: Some(file_uri.to_string()),
                range: None,
                selection_range: None,
                container_node_id: None,
                properties_json: json!({ "kind": "file" }),
                run_id: Some(second_run_id),
            }],
            occurrences: Vec::new(),
            edges: Vec::new(),
            edge_evidence: Vec::new(),
            route_observations: vec![DocumentSymbolWriteBatchObservationInput {
                workspace_id,
                run_id: second_run_id,
                route: "rust.document_symbols".to_string(),
                scope: "file".to_string(),
                scope_key: file_uri.to_string(),
                provider: "rust-analyzer".to_string(),
                entity_kind: "node".to_string(),
                entity_id: file_node_id.clone(),
                source_file_uri: Some(file_uri.to_string()),
                properties_json: json!({ "source": "textDocument/documentSymbol" }),
            }],
            route_status_completes: vec![DocumentSymbolWriteBatchRouteStatusCompleteInput {
                workspace_id,
                route: "rust.document_symbols".to_string(),
                scope: "file".to_string(),
                scope_key: file_uri.to_string(),
                provider: "rust-analyzer".to_string(),
                provider_version: Some("test".to_string()),
                content_hash: Some("hash-2".to_string()),
                run_id: second_run_id,
                diagnostics_json: json!({ "nodes": 1, "contains_edges": 0 }),
            }],
            close_stale_nodes: vec![DocumentSymbolWriteBatchCloseStaleRouteInput {
                workspace_id,
                run_id: second_run_id,
                route: "rust.document_symbols".to_string(),
                scope: "file".to_string(),
                scope_key: file_uri.to_string(),
                provider: "rust-analyzer".to_string(),
            }],
            close_stale_edges: vec![DocumentSymbolWriteBatchCloseStaleRouteInput {
                workspace_id,
                run_id: second_run_id,
                route: "rust.document_symbols".to_string(),
                scope: "file".to_string(),
                scope_key: file_uri.to_string(),
                provider: "rust-analyzer".to_string(),
            }],
        })
        .await?;
    writer.finish_run(second_run_id, "complete").await?;
    writer.shutdown().await?;

    assert_eq!(summary.stale_nodes_closed, 0);
    assert_eq!(summary.stale_edges_closed, 0);
    assert_eq!(second_summary.stale_nodes_closed, 1);
    assert_eq!(second_summary.stale_edges_closed, 1);

    let pool = sqlite_pool(&path).await?;
    let file_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM files WHERE uri = ?")
        .bind(file_uri)
        .fetch_one(&pool)
        .await?;
    let node_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM nodes WHERE file_id IN (SELECT id FROM files WHERE uri = ?)",
    )
    .bind(file_uri)
    .fetch_one(&pool)
    .await?;
    let occurrence_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM occurrences WHERE run_id = ?")
            .bind(run_id)
            .fetch_one(&pool)
            .await?;
    let edge_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM edges WHERE id = ?")
        .bind(&contains_edge_id)
        .fetch_one(&pool)
        .await?;
    let evidence_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM edge_evidence WHERE edge_id = ?")
            .bind(&contains_edge_id)
            .fetch_one(&pool)
            .await?;
    let route_status: String = sqlx::query_scalar(
        "SELECT last_status FROM extraction_route_status WHERE route = 'rust.document_symbols'",
    )
    .fetch_one(&pool)
    .await?;
    let file_node_valid_to: Option<i64> =
        sqlx::query_scalar("SELECT valid_to_run_id FROM nodes WHERE id = ?")
            .bind(&file_node_id)
            .fetch_one(&pool)
            .await?;
    let function_node_valid_to: Option<i64> =
        sqlx::query_scalar("SELECT valid_to_run_id FROM nodes WHERE id = ?")
            .bind(&function_node_id)
            .fetch_one(&pool)
            .await?;
    let contains_edge_valid_to: Option<i64> =
        sqlx::query_scalar("SELECT valid_to_run_id FROM edges WHERE id = ?")
            .bind(&contains_edge_id)
            .fetch_one(&pool)
            .await?;

    assert_eq!(file_count, 1);
    assert_eq!(node_count, 2);
    assert_eq!(occurrence_count, 1);
    assert_eq!(edge_count, 1);
    assert_eq!(evidence_count, 1);
    assert_eq!(route_status, "complete");
    assert_eq!(file_node_valid_to, None);
    assert_eq!(function_node_valid_to, Some(second_run_id));
    assert_eq!(contains_edge_valid_to, Some(second_run_id));

    Ok(())
}

#[tokio::test]
async fn close_stale_file_marks_file_nodes_and_edges() -> Result<(), Box<dyn Error>> {
    let path = temp_db_path()?;
    let writer = WriteManager::start(&path).await?;
    writer.migrate().await?;

    let workspace_id = writer
        .create_workspace("file:///tmp/db-manager-stale-file", "rust")
        .await?;
    let first_run_id = writer
        .start_run(workspace_id, "rust-analyzer", Some("test"), None)
        .await?;
    let file_uri = "file:///tmp/db-manager-stale-file/src/lib.rs";
    let file_id = writer
        .upsert_file(FileInput {
            workspace_id,
            uri: file_uri,
            path: "src/lib.rs",
            language: "rust",
            content_hash: None,
            last_seen_run_id: Some(first_run_id),
            properties_json: json!({}),
        })
        .await?;
    let source_symbol_key = format!("{file_uri}#function:source:1:0");
    let target_symbol_key = "file:///tmp/db-manager-stale-file/src/target.rs#function:target:1:0";
    let source_node_id = writer
        .upsert_node(NodeInput {
            workspace_id,
            language: "rust",
            kind: "function",
            name: "source",
            qualified_name: Some("source"),
            display_name: Some("source"),
            symbol_key: &source_symbol_key,
            file_id: Some(file_id),
            range: None,
            selection_range: None,
            container_node_id: None,
            properties_json: json!({}),
            run_id: Some(first_run_id),
        })
        .await?;
    let target_node_id = writer
        .upsert_node(NodeInput {
            workspace_id,
            language: "rust",
            kind: "function",
            name: "target",
            qualified_name: Some("target"),
            display_name: Some("target"),
            symbol_key: target_symbol_key,
            file_id: None,
            range: None,
            selection_range: None,
            container_node_id: None,
            properties_json: json!({}),
            run_id: Some(first_run_id),
        })
        .await?;
    let edge_id = writer
        .upsert_edge(EdgeInput {
            workspace_id,
            src_node_id: &source_node_id,
            dst_node_id: &target_node_id,
            relation: "calls",
            context: Some("symbol"),
            confidence: "EXTRACTED",
            confidence_score: 1.0,
            weight: 1.0,
            properties_json: json!({}),
            run_id: Some(first_run_id),
        })
        .await?;
    writer
        .insert_edge_evidence(EdgeEvidenceInput {
            edge_id: &edge_id,
            run_id: first_run_id,
            provider: "rust-analyzer",
            lsp_method: Some("callHierarchy/outgoingCalls"),
            file_id: Some(file_id),
            range: None,
            raw_json: Some(json!({})),
        })
        .await?;
    writer.finish_run(first_run_id, "complete").await?;

    let stale_run_id = writer
        .start_run(workspace_id, "rust-analyzer", Some("test"), None)
        .await?;
    let summary = writer
        .close_stale_file(CloseStaleFileInput {
            workspace_id,
            run_id: stale_run_id,
            file_uri,
        })
        .await?;
    writer.finish_run(stale_run_id, "complete").await?;
    writer.shutdown().await?;

    assert_eq!(summary.file_id, Some(file_id));
    assert_eq!(summary.stale_nodes_closed, 1);
    assert_eq!(summary.stale_edges_closed, 1);

    let pool = sqlite_pool(&path).await?;
    let source_node_valid_to: Option<i64> =
        sqlx::query_scalar("SELECT valid_to_run_id FROM nodes WHERE id = ?")
            .bind(&source_node_id)
            .fetch_one(&pool)
            .await?;
    let target_node_valid_to: Option<i64> =
        sqlx::query_scalar("SELECT valid_to_run_id FROM nodes WHERE id = ?")
            .bind(&target_node_id)
            .fetch_one(&pool)
            .await?;
    let edge_valid_to: Option<i64> =
        sqlx::query_scalar("SELECT valid_to_run_id FROM edges WHERE id = ?")
            .bind(&edge_id)
            .fetch_one(&pool)
            .await?;

    assert_eq!(source_node_valid_to, Some(stale_run_id));
    assert_eq!(target_node_valid_to, None);
    assert_eq!(edge_valid_to, Some(stale_run_id));

    Ok(())
}

#[tokio::test]
async fn file_route_stale_pass_closes_workspace_source_file_observations()
-> Result<(), Box<dyn Error>> {
    let path = temp_db_path()?;
    let writer = WriteManager::start(&path).await?;
    writer.migrate().await?;

    let workspace_uri = "file:///tmp/db-manager-file-route-stale";
    let workspace_id = writer.create_workspace(workspace_uri, "rust").await?;
    let file_uri = "file:///tmp/db-manager-file-route-stale/src/lib.rs";
    let previous_run_id = writer
        .start_run(workspace_id, "rust-analyzer", Some("test"), None)
        .await?;
    let file_id = writer
        .upsert_file(FileInput {
            workspace_id,
            uri: file_uri,
            path: "src/lib.rs",
            language: "rust",
            content_hash: None,
            last_seen_run_id: Some(previous_run_id),
            properties_json: json!({}),
        })
        .await?;
    let caller_file_uri = "file:///tmp/db-manager-file-route-stale/src/caller.rs";
    let caller_file_id = writer
        .upsert_file(FileInput {
            workspace_id,
            uri: caller_file_uri,
            path: "src/caller.rs",
            language: "rust",
            content_hash: None,
            last_seen_run_id: Some(previous_run_id),
            properties_json: json!({}),
        })
        .await?;

    let stale_symbol_key = format!("{file_uri}#function:stale:1:0");
    let current_symbol_key = format!("{file_uri}#function:current:5:0");
    let caller_symbol_key = format!("{caller_file_uri}#function:caller:1:0");
    let target_symbol_key =
        "file:///tmp/db-manager-file-route-stale/src/target.rs#function:target:1:0";
    let stale_node_id = writer
        .upsert_node(NodeInput {
            workspace_id,
            language: "rust",
            kind: "function",
            name: "stale",
            qualified_name: Some("stale"),
            display_name: Some("stale"),
            symbol_key: &stale_symbol_key,
            file_id: Some(file_id),
            range: None,
            selection_range: None,
            container_node_id: None,
            properties_json: json!({}),
            run_id: Some(previous_run_id),
        })
        .await?;
    let current_node_id = writer
        .upsert_node(NodeInput {
            workspace_id,
            language: "rust",
            kind: "function",
            name: "current",
            qualified_name: Some("current"),
            display_name: Some("current"),
            symbol_key: &current_symbol_key,
            file_id: Some(file_id),
            range: None,
            selection_range: None,
            container_node_id: None,
            properties_json: json!({}),
            run_id: Some(previous_run_id),
        })
        .await?;
    let target_node_id = writer
        .upsert_node(NodeInput {
            workspace_id,
            language: "rust",
            kind: "function",
            name: "target",
            qualified_name: Some("target"),
            display_name: Some("target"),
            symbol_key: target_symbol_key,
            file_id: None,
            range: None,
            selection_range: None,
            container_node_id: None,
            properties_json: json!({}),
            run_id: Some(previous_run_id),
        })
        .await?;
    let caller_node_id = writer
        .upsert_node(NodeInput {
            workspace_id,
            language: "rust",
            kind: "function",
            name: "caller",
            qualified_name: Some("caller"),
            display_name: Some("caller"),
            symbol_key: &caller_symbol_key,
            file_id: Some(caller_file_id),
            range: None,
            selection_range: None,
            container_node_id: None,
            properties_json: json!({}),
            run_id: Some(previous_run_id),
        })
        .await?;
    let stale_edge_id = writer
        .upsert_edge(EdgeInput {
            workspace_id,
            src_node_id: &stale_node_id,
            dst_node_id: &target_node_id,
            relation: "references",
            context: Some("symbol"),
            confidence: "EXTRACTED",
            confidence_score: 1.0,
            weight: 1.0,
            properties_json: json!({}),
            run_id: Some(previous_run_id),
        })
        .await?;
    let current_edge_id = writer
        .upsert_edge(EdgeInput {
            workspace_id,
            src_node_id: &current_node_id,
            dst_node_id: &target_node_id,
            relation: "references",
            context: Some("symbol"),
            confidence: "EXTRACTED",
            confidence_score: 1.0,
            weight: 1.0,
            properties_json: json!({}),
            run_id: Some(previous_run_id),
        })
        .await?;
    let inbound_edge_id = writer
        .upsert_edge(EdgeInput {
            workspace_id,
            src_node_id: &caller_node_id,
            dst_node_id: &stale_node_id,
            relation: "references",
            context: Some("symbol"),
            confidence: "EXTRACTED",
            confidence_score: 1.0,
            weight: 1.0,
            properties_json: json!({}),
            run_id: Some(previous_run_id),
        })
        .await?;

    for (route, entity_kind, entity_id, source_file_id) in [
        (
            "rust.document_symbols",
            "node",
            stale_node_id.as_str(),
            file_id,
        ),
        (
            "rust.document_symbols",
            "node",
            current_node_id.as_str(),
            file_id,
        ),
        ("rust.references", "edge", stale_edge_id.as_str(), file_id),
        ("rust.references", "edge", current_edge_id.as_str(), file_id),
        (
            "rust.references",
            "edge",
            inbound_edge_id.as_str(),
            caller_file_id,
        ),
    ] {
        writer
            .record_route_observation(RouteObservationInput {
                workspace_id,
                run_id: previous_run_id,
                route,
                scope: "workspace",
                scope_key: workspace_uri,
                provider: "rust-analyzer",
                entity_kind,
                entity_id,
                source_file_id: Some(source_file_id),
                properties_json: json!({}),
            })
            .await?;
    }
    writer.finish_run(previous_run_id, "complete").await?;

    let current_run_id = writer
        .start_run(workspace_id, "rust-analyzer", Some("test"), None)
        .await?;
    for route in ["rust.document_symbols", "rust.references"] {
        writer
            .start_route_status(RouteStatusStartInput {
                workspace_id,
                route,
                scope: "file",
                scope_key: file_uri,
                file_id: Some(file_id),
                provider: "rust-analyzer",
                provider_version: Some("test"),
                content_hash: None,
                run_id: current_run_id,
                diagnostics_json: json!({}),
            })
            .await?;
    }
    writer
        .record_route_observation(RouteObservationInput {
            workspace_id,
            run_id: current_run_id,
            route: "rust.document_symbols",
            scope: "file",
            scope_key: file_uri,
            provider: "rust-analyzer",
            entity_kind: "node",
            entity_id: &current_node_id,
            source_file_id: Some(file_id),
            properties_json: json!({}),
        })
        .await?;
    writer
        .record_route_observation(RouteObservationInput {
            workspace_id,
            run_id: current_run_id,
            route: "rust.references",
            scope: "file",
            scope_key: file_uri,
            provider: "rust-analyzer",
            entity_kind: "edge",
            entity_id: &current_edge_id,
            source_file_id: Some(file_id),
            properties_json: json!({}),
        })
        .await?;
    for route in ["rust.document_symbols", "rust.references"] {
        writer
            .complete_route_status(RouteStatusCompleteInput {
                workspace_id,
                route,
                scope: "file",
                scope_key: file_uri,
                provider: "rust-analyzer",
                provider_version: Some("test"),
                content_hash: None,
                run_id: current_run_id,
                diagnostics_json: json!({}),
            })
            .await?;
    }

    let stale_nodes_closed = writer
        .close_stale_nodes_for_route(CloseStaleRouteInput {
            workspace_id,
            run_id: current_run_id,
            route: "rust.document_symbols",
            scope: "file",
            scope_key: file_uri,
            provider: "rust-analyzer",
        })
        .await?;
    let stale_edges_closed = writer
        .close_stale_edges_for_route(CloseStaleRouteInput {
            workspace_id,
            run_id: current_run_id,
            route: "rust.references",
            scope: "file",
            scope_key: file_uri,
            provider: "rust-analyzer",
        })
        .await?;
    writer.finish_run(current_run_id, "complete").await?;
    writer.shutdown().await?;

    assert_eq!(stale_nodes_closed, 1);
    assert_eq!(stale_edges_closed, 2);

    let pool = sqlite_pool(&path).await?;
    let stale_node_valid_to: Option<i64> =
        sqlx::query_scalar("SELECT valid_to_run_id FROM nodes WHERE id = ?")
            .bind(&stale_node_id)
            .fetch_one(&pool)
            .await?;
    let current_node_valid_to: Option<i64> =
        sqlx::query_scalar("SELECT valid_to_run_id FROM nodes WHERE id = ?")
            .bind(&current_node_id)
            .fetch_one(&pool)
            .await?;
    let stale_edge_valid_to: Option<i64> =
        sqlx::query_scalar("SELECT valid_to_run_id FROM edges WHERE id = ?")
            .bind(&stale_edge_id)
            .fetch_one(&pool)
            .await?;
    let inbound_edge_valid_to: Option<i64> =
        sqlx::query_scalar("SELECT valid_to_run_id FROM edges WHERE id = ?")
            .bind(&inbound_edge_id)
            .fetch_one(&pool)
            .await?;
    let current_edge_valid_to: Option<i64> =
        sqlx::query_scalar("SELECT valid_to_run_id FROM edges WHERE id = ?")
            .bind(&current_edge_id)
            .fetch_one(&pool)
            .await?;

    assert_eq!(stale_node_valid_to, Some(current_run_id));
    assert_eq!(current_node_valid_to, None);
    assert_eq!(stale_edge_valid_to, Some(current_run_id));
    assert_eq!(inbound_edge_valid_to, Some(current_run_id));
    assert_eq!(current_edge_valid_to, None);

    Ok(())
}

#[tokio::test]
async fn file_route_hashes_and_active_symbols_can_be_read() -> Result<(), Box<dyn Error>> {
    let path = temp_db_path()?;
    let writer = WriteManager::start(&path).await?;
    writer.migrate().await?;

    let workspace_uri = "file:///tmp/db-manager-active-symbols";
    let workspace_id = writer.create_workspace(workspace_uri, "rust").await?;
    let run_id = writer
        .start_run(workspace_id, "rust-analyzer", Some("test"), None)
        .await?;
    let file_uri = "file:///tmp/db-manager-active-symbols/src/lib.rs";
    let file_id = writer
        .upsert_file(FileInput {
            workspace_id,
            uri: file_uri,
            path: "src/lib.rs",
            language: "rust",
            content_hash: Some("hash-a"),
            last_seen_run_id: Some(run_id),
            properties_json: json!({
                "raw_metadata": {
                    "lsp_method": "textDocument/documentSymbol"
                }
            }),
        })
        .await?;
    let symbol_key = format!("{file_uri}#kind=function;selection=1:0-1:4;name=demo;parent=");
    writer
        .upsert_node(NodeInput {
            workspace_id,
            language: "rust",
            kind: "function",
            name: "demo",
            qualified_name: Some("demo"),
            display_name: Some("demo"),
            symbol_key: &symbol_key,
            file_id: Some(file_id),
            range: Some(TextRange {
                start_line: 1,
                start_col: 0,
                end_line: 3,
                end_col: 1,
            }),
            selection_range: Some(TextRange {
                start_line: 1,
                start_col: 0,
                end_line: 1,
                end_col: 4,
            }),
            container_node_id: None,
            properties_json: json!({
                "detail": "fn()",
                "raw": {
                    "document_symbol": {
                        "selectionRange": {
                            "start": { "line": 1, "character": 0 },
                            "end": { "line": 1, "character": 4 }
                        }
                    }
                }
            }),
            run_id: Some(run_id),
        })
        .await?;
    writer
        .start_route_status(RouteStatusStartInput {
            workspace_id,
            route: "rust.document_symbols",
            scope: "file",
            scope_key: file_uri,
            file_id: Some(file_id),
            provider: "rust-analyzer",
            provider_version: Some("test"),
            content_hash: Some("hash-a"),
            run_id,
            diagnostics_json: json!({}),
        })
        .await?;
    writer
        .complete_route_status(RouteStatusCompleteInput {
            workspace_id,
            route: "rust.document_symbols",
            scope: "file",
            scope_key: file_uri,
            provider: "rust-analyzer",
            provider_version: Some("test"),
            content_hash: Some("hash-a"),
            run_id,
            diagnostics_json: json!({}),
        })
        .await?;

    let hashes = writer
        .file_route_content_hashes(workspace_id, "rust.document_symbols", "rust-analyzer")
        .await?;
    let active_files = writer
        .active_file_symbols(workspace_id, "rust", &[file_uri.to_string()])
        .await?;
    writer.finish_run(run_id, "complete").await?;
    writer.shutdown().await?;

    assert_eq!(hashes.get(file_uri), Some(&Some("hash-a".to_string())));
    assert_eq!(active_files.len(), 1);
    assert_eq!(active_files[0].uri, file_uri);
    assert_eq!(active_files[0].symbols.len(), 1);
    assert_eq!(active_files[0].symbols[0].symbol_key, symbol_key);

    Ok(())
}

#[tokio::test]
async fn source_file_stale_close_does_not_close_inbound_edges() -> Result<(), Box<dyn Error>> {
    let path = temp_db_path()?;
    let writer = WriteManager::start(&path).await?;
    writer.migrate().await?;

    let workspace_uri = "file:///tmp/db-manager-source-file-close";
    let workspace_id = writer.create_workspace(workspace_uri, "rust").await?;
    let previous_run_id = writer
        .start_run(workspace_id, "rust-analyzer", Some("test"), None)
        .await?;
    let current_run_id = writer
        .start_run(workspace_id, "rust-analyzer", Some("test"), None)
        .await?;
    let file_a_uri = "file:///tmp/db-manager-source-file-close/src/a.rs";
    let file_b_uri = "file:///tmp/db-manager-source-file-close/src/b.rs";
    let file_c_uri = "file:///tmp/db-manager-source-file-close/src/c.rs";
    let file_a_id = writer
        .upsert_file(FileInput {
            workspace_id,
            uri: file_a_uri,
            path: "src/a.rs",
            language: "rust",
            content_hash: Some("hash-a"),
            last_seen_run_id: Some(previous_run_id),
            properties_json: json!({}),
        })
        .await?;
    let file_b_id = writer
        .upsert_file(FileInput {
            workspace_id,
            uri: file_b_uri,
            path: "src/b.rs",
            language: "rust",
            content_hash: Some("hash-b"),
            last_seen_run_id: Some(previous_run_id),
            properties_json: json!({}),
        })
        .await?;
    let file_c_id = writer
        .upsert_file(FileInput {
            workspace_id,
            uri: file_c_uri,
            path: "src/c.rs",
            language: "rust",
            content_hash: Some("hash-c"),
            last_seen_run_id: Some(previous_run_id),
            properties_json: json!({}),
        })
        .await?;
    let a_symbol_key = format!("{file_a_uri}#function:a");
    let b_symbol_key = format!("{file_b_uri}#function:b");
    let c_symbol_key = format!("{file_c_uri}#function:c");
    let a_node_id = writer
        .upsert_node(NodeInput {
            workspace_id,
            language: "rust",
            kind: "function",
            name: "a",
            qualified_name: Some("a"),
            display_name: Some("a"),
            symbol_key: &a_symbol_key,
            file_id: Some(file_a_id),
            range: None,
            selection_range: None,
            container_node_id: None,
            properties_json: json!({}),
            run_id: Some(previous_run_id),
        })
        .await?;
    let b_node_id = writer
        .upsert_node(NodeInput {
            workspace_id,
            language: "rust",
            kind: "function",
            name: "b",
            qualified_name: Some("b"),
            display_name: Some("b"),
            symbol_key: &b_symbol_key,
            file_id: Some(file_b_id),
            range: None,
            selection_range: None,
            container_node_id: None,
            properties_json: json!({}),
            run_id: Some(previous_run_id),
        })
        .await?;
    let c_node_id = writer
        .upsert_node(NodeInput {
            workspace_id,
            language: "rust",
            kind: "function",
            name: "c",
            qualified_name: Some("c"),
            display_name: Some("c"),
            symbol_key: &c_symbol_key,
            file_id: Some(file_c_id),
            range: None,
            selection_range: None,
            container_node_id: None,
            properties_json: json!({}),
            run_id: Some(previous_run_id),
        })
        .await?;
    let stale_origin_edge_id = writer
        .upsert_edge(EdgeInput {
            workspace_id,
            src_node_id: &a_node_id,
            dst_node_id: &b_node_id,
            relation: "calls",
            context: Some("direct"),
            confidence: "EXTRACTED",
            confidence_score: 1.0,
            weight: 1.0,
            properties_json: json!({}),
            run_id: Some(previous_run_id),
        })
        .await?;
    let inbound_edge_id = writer
        .upsert_edge(EdgeInput {
            workspace_id,
            src_node_id: &c_node_id,
            dst_node_id: &a_node_id,
            relation: "calls",
            context: Some("direct"),
            confidence: "EXTRACTED",
            confidence_score: 1.0,
            weight: 1.0,
            properties_json: json!({}),
            run_id: Some(previous_run_id),
        })
        .await?;
    writer
        .record_route_observation(RouteObservationInput {
            workspace_id,
            run_id: previous_run_id,
            route: "rust.calls",
            scope: "workspace",
            scope_key: workspace_uri,
            provider: "rust-analyzer",
            entity_kind: "edge",
            entity_id: &stale_origin_edge_id,
            source_file_id: Some(file_a_id),
            properties_json: json!({}),
        })
        .await?;
    writer
        .record_route_observation(RouteObservationInput {
            workspace_id,
            run_id: previous_run_id,
            route: "rust.calls",
            scope: "workspace",
            scope_key: workspace_uri,
            provider: "rust-analyzer",
            entity_kind: "edge",
            entity_id: &inbound_edge_id,
            source_file_id: Some(file_c_id),
            properties_json: json!({}),
        })
        .await?;
    writer
        .start_route_status(RouteStatusStartInput {
            workspace_id,
            route: "rust.calls",
            scope: "file",
            scope_key: file_a_uri,
            file_id: Some(file_a_id),
            provider: "rust-analyzer",
            provider_version: Some("test"),
            content_hash: Some("hash-a"),
            run_id: current_run_id,
            diagnostics_json: json!({}),
        })
        .await?;
    writer
        .complete_route_status(RouteStatusCompleteInput {
            workspace_id,
            route: "rust.calls",
            scope: "file",
            scope_key: file_a_uri,
            provider: "rust-analyzer",
            provider_version: Some("test"),
            content_hash: Some("hash-a"),
            run_id: current_run_id,
            diagnostics_json: json!({}),
        })
        .await?;

    let stale_edges_closed = writer
        .close_stale_edges_for_route_source_file(CloseStaleRouteInput {
            workspace_id,
            run_id: current_run_id,
            route: "rust.calls",
            scope: "file",
            scope_key: file_a_uri,
            provider: "rust-analyzer",
        })
        .await?;
    writer.finish_run(previous_run_id, "complete").await?;
    writer.finish_run(current_run_id, "complete").await?;
    writer.shutdown().await?;

    assert_eq!(stale_edges_closed, 1);

    let pool = sqlite_pool(&path).await?;
    let stale_origin_valid_to: Option<i64> =
        sqlx::query_scalar("SELECT valid_to_run_id FROM edges WHERE id = ?")
            .bind(&stale_origin_edge_id)
            .fetch_one(&pool)
            .await?;
    let inbound_valid_to: Option<i64> =
        sqlx::query_scalar("SELECT valid_to_run_id FROM edges WHERE id = ?")
            .bind(&inbound_edge_id)
            .fetch_one(&pool)
            .await?;

    assert_eq!(stale_origin_valid_to, Some(current_run_id));
    assert_eq!(inbound_valid_to, None);

    Ok(())
}

async fn sqlite_pool(path: &Path) -> Result<SqlitePool, Box<dyn Error>> {
    Ok(SqlitePool::connect(&format!("sqlite://{}", path.display())).await?)
}

fn temp_db_path() -> Result<PathBuf, Box<dyn Error>> {
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    Ok(env::temp_dir().join(format!(
        "poc-semanticgraph-db-manager-{}-{stamp}.db",
        std::process::id()
    )))
}

fn sidecar_path(path: &Path, suffix: &str) -> PathBuf {
    let mut path = path.as_os_str().to_os_string();
    path.push(format!("-{suffix}"));
    PathBuf::from(path)
}
