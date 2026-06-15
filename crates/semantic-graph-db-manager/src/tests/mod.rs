use crate::{
    CloseStaleFileInput, CloseStaleRouteInput, Config, EdgeEvidenceInput, EdgeInput, FileInput,
    NodeInput, RouteObservationInput, RouteStatusCompleteInput, RouteStatusStartInput,
    WriteManager, edge_id, node_id,
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
