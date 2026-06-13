use std::env;
use std::error::Error;
use std::io;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use lsp_types::DocumentSymbolResponse;
use semantic_graph_extract::document_symbols::paths::file_uri;
use semantic_graph_extract::error::ExtractError;
use semantic_graph_extract::model::DocumentSymbolRequest;
use semantic_graph_extract::persist::ExtractionPersister;
use semantic_graph_extract::providers::rust_analyzer::RustDocumentSymbolMapper;
use semantic_graph_store::{GraphStore, GraphStoreStats};
use serde_json::json;
use sqlx::SqlitePool;

fn request() -> std::result::Result<DocumentSymbolRequest, Box<dyn Error>> {
    let cwd = repo_root()?;

    Ok(DocumentSymbolRequest {
        workspace_root: cwd.clone(),
        package_path: cwd.join("crates/wip"),
        file_path: cwd.join("crates/wip/src/lib.rs"),
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

fn temp_db_path() -> std::result::Result<PathBuf, Box<dyn Error>> {
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    Ok(env::temp_dir().join(format!(
        "poc-semanticgraph-extract-{}-{stamp}.db",
        std::process::id()
    )))
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
    let store = GraphStore::connect(&db_path).await?;
    store.migrate().await?;
    let workspace_root_uri = file_uri(&repo_root()?)?;
    let extraction =
        RustDocumentSymbolMapper::map_response(request()?, fixture_response()?, None, json!({}))?;

    let summary = ExtractionPersister
        .persist_document_symbols(&store, &workspace_root_uri, &extraction)
        .await?;

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
