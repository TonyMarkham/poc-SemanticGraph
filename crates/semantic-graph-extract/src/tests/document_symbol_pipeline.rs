use std::env;
use std::error::Error;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::document_symbols::paths::{file_uri, validate_document_symbol_batch_request};
use crate::error::ExtractError;
use crate::model::{
    DocumentSymbolBatchExtraction, DocumentSymbolBatchRequest, DocumentSymbolRequest, ProviderId,
};
use crate::persist::ExtractionPersister;
use crate::providers::rust_analyzer::RustDocumentSymbolMapper;
use lsp_types::DocumentSymbolResponse;
use semantic_graph_db_manager::WriteManager;
use semantic_graph_store::{GraphStore, GraphStoreStats};
use serde_json::json;
use sqlx::SqlitePool;

fn request() -> std::result::Result<DocumentSymbolRequest, Box<dyn Error>> {
    request_for("crates/wip/src/lib.rs")
}

fn request_for(relative_path: &str) -> std::result::Result<DocumentSymbolRequest, Box<dyn Error>> {
    let cwd = repo_root()?;

    Ok(DocumentSymbolRequest {
        workspace_root: cwd.clone(),
        package_path: cwd.join("crates/wip"),
        file_path: cwd.join(relative_path),
    })
}

fn repo_root() -> std::result::Result<PathBuf, Box<dyn Error>> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let crates_dir = manifest_dir
        .parent()
        .ok_or_else(|| io::Error::other("extract crate manifest dir has no parent directory"))?;
    let repo_root = crates_dir
        .parent()
        .ok_or_else(|| io::Error::other("crates directory has no parent directory"))?;

    Ok(repo_root.to_path_buf())
}

fn fixture_response() -> std::result::Result<DocumentSymbolResponse, Box<dyn Error>> {
    let fixture = include_str!("fixtures/rust_document_symbols_lib.json");
    Ok(serde_json::from_str(fixture)?)
}

fn models_fixture_response() -> std::result::Result<DocumentSymbolResponse, Box<dyn Error>> {
    let fixture = include_str!("fixtures/rust_document_symbols_models.json");
    Ok(serde_json::from_str(fixture)?)
}

fn pipeline_fixture_response() -> std::result::Result<DocumentSymbolResponse, Box<dyn Error>> {
    let fixture = include_str!("fixtures/rust_document_symbols_pipeline.json");
    Ok(serde_json::from_str(fixture)?)
}

fn batch_fixture_extraction() -> std::result::Result<DocumentSymbolBatchExtraction, Box<dyn Error>>
{
    let provider_version = Some("fixture-rust-analyzer".to_string());
    let extractions = vec![
        RustDocumentSymbolMapper::map_response(
            request_for("crates/wip/src/lib.rs")?,
            fixture_response()?,
            provider_version.clone(),
            json!({ "fixture": "lib.rs" }),
        )?,
        RustDocumentSymbolMapper::map_response(
            request_for("crates/wip/src/models.rs")?,
            models_fixture_response()?,
            provider_version.clone(),
            json!({ "fixture": "models.rs" }),
        )?,
        RustDocumentSymbolMapper::map_response(
            request_for("crates/wip/src/pipeline.rs")?,
            pipeline_fixture_response()?,
            provider_version.clone(),
            json!({ "fixture": "pipeline.rs" }),
        )?,
    ];

    Ok(DocumentSymbolBatchExtraction {
        provider: ProviderId::rust_analyzer(),
        provider_version,
        extractions,
        raw_metadata: json!({ "fixture": "crate batch" }),
    })
}

fn temp_db_path() -> std::result::Result<PathBuf, Box<dyn Error>> {
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    Ok(env::temp_dir().join(format!(
        "poc-semanticgraph-extract-{}-{stamp}.db",
        std::process::id()
    )))
}

fn temp_workspace_path(name: &str) -> std::result::Result<PathBuf, Box<dyn Error>> {
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    Ok(env::temp_dir().join(format!(
        "poc-semanticgraph-extract-{name}-{}-{stamp}",
        std::process::id()
    )))
}

#[test]
fn batch_validation_rejects_files_outside_workspace_root() -> std::result::Result<(), Box<dyn Error>>
{
    let workspace_root = temp_workspace_path("workspace")?;
    let package_path = workspace_root.join("crates/example");
    let outside_root = temp_workspace_path("outside")?;
    fs::create_dir_all(&package_path)?;
    fs::create_dir_all(&outside_root)?;
    let outside_file = outside_root.join("outside.rs");
    fs::write(&outside_file, "")?;

    let result = validate_document_symbol_batch_request(DocumentSymbolBatchRequest {
        workspace_root,
        package_path,
        file_paths: vec![outside_file],
    });

    assert!(matches!(result, Err(ExtractError::InvalidPath { .. })));
    Ok(())
}

#[test]
fn maps_hierarchical_fixture_to_provider_neutral_records() -> std::result::Result<(), Box<dyn Error>>
{
    let extraction =
        RustDocumentSymbolMapper::map_response(request()?, fixture_response()?, None, json!({}))?;

    assert_eq!(extraction.provider.as_str(), "rust-analyzer");
    assert_eq!(
        extraction.source_file.relative_path,
        "crates/wip/src/lib.rs"
    );
    assert_eq!(extraction.symbols.len(), 4);
    assert_eq!(extraction.relations.len(), 4);
    assert!(
        extraction
            .symbols
            .iter()
            .any(|symbol| symbol.kind == "function"
                && symbol.qualified_name.as_deref()
                    == Some("tests::processor_tracks_active_widgets"))
    );

    let first_key = extraction.symbols[0].symbol_key.clone();
    let second_extraction =
        RustDocumentSymbolMapper::map_response(request()?, fixture_response()?, None, json!({}))?;
    assert_eq!(first_key, second_extraction.symbols[0].symbol_key);
    Ok(())
}

#[tokio::test]
async fn persists_fixture_symbols_into_sqlite() -> std::result::Result<(), Box<dyn Error>> {
    let db_path = temp_db_path()?;
    let writer = WriteManager::start(&db_path).await?;
    writer.migrate().await?;
    let workspace_root_uri = file_uri(&repo_root()?)?;
    let extraction =
        RustDocumentSymbolMapper::map_response(request()?, fixture_response()?, None, json!({}))?;

    let summary = ExtractionPersister
        .persist_document_symbols(&writer, &workspace_root_uri, &extraction)
        .await?;
    writer.shutdown().await?;
    let store = GraphStore::connect(&db_path).await?;

    assert_eq!(summary.files, 1);
    assert_eq!(summary.nodes, 5);
    assert_eq!(summary.edges, 4);
    assert_eq!(summary.occurrences, 4);
    assert_eq!(summary.evidence, 4);
    assert_eq!(
        store.stats().await?,
        GraphStoreStats {
            workspaces: 1,
            extraction_runs: 1,
            files: 1,
            nodes: 5,
            edges: 4,
            occurrences: 4,
            edge_evidence: 4,
        }
    );

    let pool = SqlitePool::connect(&format!("sqlite://{}", db_path.display())).await?;
    let file_nodes: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM nodes WHERE kind = 'file'")
        .fetch_one(&pool)
        .await?;
    let definitions: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM occurrences WHERE role = 'definition'")
            .fetch_one(&pool)
            .await?;
    let contains_edges: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM edges WHERE relation = 'contains' AND confidence = 'EXTRACTED' AND confidence_score = 1.0",
    )
    .fetch_one(&pool)
    .await?;
    let evidence: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM edge_evidence WHERE lsp_method = 'textDocument/documentSymbol'",
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(file_nodes, 1);
    assert_eq!(definitions, 4);
    assert_eq!(contains_edges, 4);
    assert_eq!(evidence, 4);
    Ok(())
}

#[tokio::test]
async fn persists_batch_fixture_symbols_into_one_sqlite_run()
-> std::result::Result<(), Box<dyn Error>> {
    let db_path = temp_db_path()?;
    let writer = WriteManager::start(&db_path).await?;
    writer.migrate().await?;
    let workspace_root_uri = file_uri(&repo_root()?)?;
    let extraction = batch_fixture_extraction()?;

    let summary = ExtractionPersister
        .persist_document_symbol_batch(&writer, &workspace_root_uri, &extraction)
        .await?;
    writer.shutdown().await?;
    let store = GraphStore::connect(&db_path).await?;

    assert_eq!(summary.files, 3);
    assert_eq!(summary.nodes, 27);
    assert_eq!(summary.edges, 24);
    assert_eq!(summary.occurrences, 24);
    assert_eq!(summary.evidence, 24);
    assert_eq!(
        store.stats().await?,
        GraphStoreStats {
            workspaces: 1,
            extraction_runs: 1,
            files: 3,
            nodes: 27,
            edges: 24,
            occurrences: 24,
            edge_evidence: 24,
        }
    );

    let pool = SqlitePool::connect(&format!("sqlite://{}", db_path.display())).await?;
    let file_rows: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM files
        WHERE path IN (
          'crates/wip/src/lib.rs',
          'crates/wip/src/models.rs',
          'crates/wip/src/pipeline.rs'
        )
        "#,
    )
    .fetch_one(&pool)
    .await?;
    let file_nodes: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM nodes WHERE kind = 'file'")
        .fetch_one(&pool)
        .await?;
    let model_file_contains_widget_id: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM edges
        JOIN nodes src ON src.id = edges.src_node_id
        JOIN nodes dst ON dst.id = edges.dst_node_id
        WHERE edges.relation = 'contains'
          AND src.kind = 'file'
          AND src.qualified_name = 'crates/wip/src/models.rs'
          AND dst.name = 'WidgetId'
        "#,
    )
    .fetch_one(&pool)
    .await?;
    let nested_new_on_widget_id: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM nodes child
        JOIN nodes parent ON parent.id = child.container_node_id
        WHERE child.name = 'new'
          AND parent.name = 'WidgetId'
          AND parent.qualified_name = 'WidgetId'
        "#,
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(file_rows, 3);
    assert_eq!(file_nodes, 3);
    assert_eq!(model_file_contains_widget_id, 1);
    assert_eq!(nested_new_on_widget_id, 1);
    Ok(())
}

#[tokio::test]
async fn deleted_rust_file_marks_file_symbols_stale() -> std::result::Result<(), Box<dyn Error>> {
    let db_path = temp_db_path()?;
    let writer = WriteManager::start(&db_path).await?;
    writer.migrate().await?;
    let workspace_root_uri = file_uri(&repo_root()?)?;
    let extraction =
        RustDocumentSymbolMapper::map_response(request()?, fixture_response()?, None, json!({}))?;

    ExtractionPersister
        .persist_document_symbols(&writer, &workspace_root_uri, &extraction)
        .await?;
    let summary = ExtractionPersister
        .mark_deleted_rust_file_stale(&writer, &workspace_root_uri, &extraction.source_file.uri)
        .await?;
    writer.shutdown().await?;

    assert_eq!(summary.routes_complete, 3);
    assert_eq!(summary.stale_nodes_closed, 5);
    assert_eq!(summary.stale_edges_closed, 4);

    let pool = SqlitePool::connect(&format!("sqlite://{}", db_path.display())).await?;
    let stale_file_nodes: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM nodes WHERE kind = 'file' AND valid_to_run_id IS NOT NULL",
    )
    .fetch_one(&pool)
    .await?;
    let stale_symbols: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM nodes WHERE kind <> 'file' AND valid_to_run_id IS NOT NULL",
    )
    .fetch_one(&pool)
    .await?;
    let stale_contains_edges: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM edges WHERE relation = 'contains' AND valid_to_run_id IS NOT NULL",
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(stale_file_nodes, 1);
    assert_eq!(stale_symbols, 4);
    assert_eq!(stale_contains_edges, 4);

    Ok(())
}

#[test]
fn provider_error_format_includes_method_context() {
    let error = ExtractError::protocol(
        "rust-analyzer",
        "textDocument/documentSymbol",
        Some(42),
        "server returned an error",
    )
    .to_string();

    assert!(error.contains("rust-analyzer"));
    assert!(error.contains("textDocument/documentSymbol"));
    assert!(error.contains("42"));
}
