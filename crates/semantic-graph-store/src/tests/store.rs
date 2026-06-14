use std::env;
use std::error::Error;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{EdgeInput, GraphStore, GraphStoreStats, edge_id, node_id};
use serde_json::json;
use sqlx::SqlitePool;

fn temp_db_path() -> std::result::Result<PathBuf, Box<dyn Error>> {
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    Ok(env::temp_dir().join(format!(
        "poc-semanticgraph-store-{}-{stamp}.db",
        std::process::id()
    )))
}

async fn migrated_store() -> std::result::Result<GraphStore, Box<dyn Error>> {
    let path = temp_db_path()?;
    let store = GraphStore::connect(path).await?;
    store.migrate().await?;
    Ok(store)
}

#[tokio::test]
async fn migration_creates_empty_core_schema() -> std::result::Result<(), Box<dyn Error>> {
    let path = temp_db_path()?;
    let store = GraphStore::connect(&path).await?;
    store.migrate().await?;

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
            'idx_edge_evidence_edge'
          )
        "#,
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(index_count, 9);

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
async fn demo_seed_inserts_core_graph_rows() -> std::result::Result<(), Box<dyn Error>> {
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
async fn upserts_do_not_duplicate_canonical_rows() -> std::result::Result<(), Box<dyn Error>> {
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
async fn foreign_keys_reject_invalid_edge_references() -> std::result::Result<(), Box<dyn Error>> {
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
