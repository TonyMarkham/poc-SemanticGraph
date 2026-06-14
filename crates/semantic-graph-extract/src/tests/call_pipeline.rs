use std::env;
use std::error::Error;
use std::future::Future;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::document_symbols::paths::file_uri;
use crate::model::CallBatchRequest;
use crate::persist::ExtractionPersister;
use crate::providers::rust_analyzer::RustAnalyzerProvider;

use semantic_graph_store::GraphStore;
use sqlx::SqlitePool;
use tokio::runtime::Builder;

static RUST_ANALYZER_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn extracts_rust_calls_from_wip() -> std::result::Result<(), Box<dyn Error>> {
    run_with_rust_analyzer(async {
        let repo_root = repo_root()?;
        let provider = RustAnalyzerProvider::new();
        let request = call_request(&provider, &repo_root)?;

        let extraction = provider.extract_rust_calls(request).await?;

        assert!(extraction.summary.callable_nodes > 0);
        assert!(extraction.summary.call_edges > 0);
        assert!(extraction.summary.call_occurrences > 0);
        assert!(extraction.calls.iter().any(|call| {
            call.occurrences.iter().any(|occurrence| {
                occurrence.file_relative_path == "crates/wip/src/pipeline.rs"
                    && occurrence.enclosing_symbol_key.contains("ingest")
            })
        }));

        Ok(())
    })
}

#[test]
fn persists_call_edges_occurrences_and_evidence() -> std::result::Result<(), Box<dyn Error>> {
    run_with_rust_analyzer(async {
        let repo_root = repo_root()?;
        let provider = RustAnalyzerProvider::new();
        let request = call_request(&provider, &repo_root)?;
        let workspace_root_uri = file_uri(&request.workspace_root)?;
        let extraction = provider.extract_rust_calls(request).await?;
        let db_path = temp_db_path()?;
        let store = GraphStore::connect(&db_path).await?;
        store.migrate().await?;
        ExtractionPersister
            .persist_document_symbol_batch(
                &store,
                &workspace_root_uri,
                &extraction.document_symbols,
            )
            .await?;

        let summary = ExtractionPersister
            .persist_call_batch(&store, &workspace_root_uri, &extraction)
            .await?;

        assert_eq!(summary.files, 0);
        assert_eq!(summary.call_edges, extraction.calls.len());
        assert_eq!(
            summary.call_occurrences,
            extraction.summary.call_occurrences
        );
        assert_eq!(summary.routes_complete, 1);

        let pool = sqlite_pool(&db_path).await?;
        let call_edges: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM edges WHERE relation = 'calls'")
                .fetch_one(&pool)
                .await?;
        let call_occurrences: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM occurrences WHERE role = 'call'")
                .fetch_one(&pool)
                .await?;
        let call_evidence: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM edge_evidence WHERE lsp_method = 'callHierarchy/outgoingCalls'",
        )
        .fetch_one(&pool)
        .await?;
        let call_observations: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM route_observations WHERE route = 'rust.calls'",
        )
        .fetch_one(&pool)
        .await?;

        assert_eq!(call_edges, i64::try_from(extraction.calls.len())?);
        assert_eq!(
            call_occurrences,
            i64::try_from(extraction.summary.call_occurrences)?
        );
        assert_eq!(
            call_evidence,
            i64::try_from(extraction.summary.call_occurrences)?
        );
        assert!(call_observations > 0);

        Ok(())
    })
}

#[test]
fn later_successful_call_run_closes_unobserved_call_edges()
-> std::result::Result<(), Box<dyn Error>> {
    run_with_rust_analyzer(async {
        let repo_root = repo_root()?;
        let provider = RustAnalyzerProvider::new();
        let request = call_request(&provider, &repo_root)?;
        let workspace_root_uri = file_uri(&request.workspace_root)?;
        let extraction = provider.extract_rust_calls(request).await?;
        assert!(!extraction.calls.is_empty());

        let db_path = temp_db_path()?;
        let store = GraphStore::connect(&db_path).await?;
        store.migrate().await?;
        ExtractionPersister
            .persist_document_symbol_batch(
                &store,
                &workspace_root_uri,
                &extraction.document_symbols,
            )
            .await?;
        ExtractionPersister
            .persist_call_batch(&store, &workspace_root_uri, &extraction)
            .await?;

        let mut second_extraction = extraction.clone();
        second_extraction.calls.clear();
        second_extraction.summary.call_edges = 0;
        second_extraction.summary.call_occurrences = 0;
        ExtractionPersister
            .persist_call_batch(&store, &workspace_root_uri, &second_extraction)
            .await?;

        let pool = sqlite_pool(&db_path).await?;
        let stale_call_edges: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM edges WHERE relation = 'calls' AND valid_to_run_id IS NOT NULL",
        )
        .fetch_one(&pool)
        .await?;

        assert_eq!(stale_call_edges, i64::try_from(extraction.calls.len())?);

        Ok(())
    })
}

fn call_request(
    provider: &RustAnalyzerProvider,
    repo_root: &Path,
) -> std::result::Result<CallBatchRequest, Box<dyn Error>> {
    let package_path = repo_root.join("crates/wip");
    let file_paths = provider.discover_rust_source_files(repo_root, &package_path)?;
    Ok(CallBatchRequest {
        workspace_root: repo_root.to_path_buf(),
        package_path,
        file_paths,
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

fn temp_db_path() -> std::result::Result<PathBuf, Box<dyn Error>> {
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    Ok(env::temp_dir().join(format!(
        "poc-semanticgraph-call-extract-{}-{stamp}.db",
        std::process::id()
    )))
}

async fn sqlite_pool(path: &Path) -> std::result::Result<SqlitePool, Box<dyn Error>> {
    Ok(SqlitePool::connect(&format!("sqlite://{}", path.display())).await?)
}

fn run_with_rust_analyzer<F>(future: F) -> std::result::Result<(), Box<dyn Error>>
where
    F: Future<Output = std::result::Result<(), Box<dyn Error>>>,
{
    let _guard = rust_analyzer_guard()?;
    let runtime = Builder::new_current_thread().enable_all().build()?;
    runtime.block_on(future)
}

fn rust_analyzer_guard() -> std::result::Result<std::sync::MutexGuard<'static, ()>, Box<dyn Error>>
{
    RUST_ANALYZER_LOCK
        .lock()
        .map_err(|_| io::Error::other("rust-analyzer test mutex was poisoned").into())
}
