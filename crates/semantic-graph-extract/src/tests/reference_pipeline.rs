use std::env;
use std::error::Error;
use std::future::Future;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::document_symbols::paths::file_uri;
use crate::model::ReferenceBatchRequest;
use crate::persist::ExtractionPersister;
use crate::providers::rust_analyzer::RustAnalyzerProvider;

use semantic_graph_db_manager::WriteManager;
use sqlx::SqlitePool;
use tokio::runtime::Builder;

static RUST_ANALYZER_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn extracts_rust_references_from_wip() -> std::result::Result<(), Box<dyn Error>> {
    run_with_rust_analyzer(async {
        let repo_root = repo_root()?;
        let provider = RustAnalyzerProvider::new();
        let request = reference_request(&provider, &repo_root)?;

        let extraction = provider.extract_rust_references(request).await?;

        assert!(extraction.summary.targets_queried > 0);
        assert!(extraction.summary.reference_edges > 0);
        assert!(extraction.summary.reference_occurrences > 0);
        assert!(
            extraction
                .references
                .iter()
                .any(
                    |reference| reference.occurrences.iter().any(|occurrence| occurrence
                        .file_relative_path
                        == "crates/wip/src/tests/mod.rs"
                        && occurrence
                            .enclosing_symbol_key
                            .as_deref()
                            .unwrap_or_default()
                            .contains("processor_tracks_active_widgets"))
                )
        );

        Ok(())
    })
}

#[test]
fn persists_reference_edges_occurrences_and_evidence() -> std::result::Result<(), Box<dyn Error>> {
    run_with_rust_analyzer(async {
        let repo_root = repo_root()?;
        let provider = RustAnalyzerProvider::new();
        let request = reference_request(&provider, &repo_root)?;
        let workspace_root_uri = file_uri(&request.workspace_root)?;
        let extraction = provider.extract_rust_references(request).await?;
        let db_path = temp_db_path()?;
        let writer = WriteManager::start(&db_path).await?;
        writer.migrate().await?;
        ExtractionPersister
            .persist_document_symbol_batch(
                &writer,
                &workspace_root_uri,
                &extraction.document_symbols,
            )
            .await?;

        let summary = ExtractionPersister
            .persist_reference_batch(&writer, &workspace_root_uri, &extraction)
            .await?;
        writer.shutdown().await?;

        assert_eq!(summary.files, 0);
        assert_eq!(summary.reference_edges, extraction.references.len());
        assert_eq!(
            summary.reference_occurrences,
            extraction.summary.reference_occurrences
        );
        assert_eq!(summary.routes_complete, 1);

        let pool = sqlite_pool(&db_path).await?;
        let reference_edges: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM edges WHERE relation = 'references'")
                .fetch_one(&pool)
                .await?;
        let reference_occurrences: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM occurrences WHERE role = 'reference'")
                .fetch_one(&pool)
                .await?;
        let reference_evidence: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM edge_evidence WHERE lsp_method = 'textDocument/references'",
        )
        .fetch_one(&pool)
        .await?;
        let reference_observations: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM route_observations WHERE route = 'rust.references'",
        )
        .fetch_one(&pool)
        .await?;

        assert_eq!(reference_edges, i64::try_from(extraction.references.len())?);
        assert_eq!(
            reference_occurrences,
            i64::try_from(extraction.summary.reference_occurrences)?
        );
        assert_eq!(
            reference_evidence,
            i64::try_from(extraction.summary.reference_occurrences)?
        );
        assert!(reference_observations > 0);

        Ok(())
    })
}

#[test]
fn later_successful_reference_run_closes_unobserved_reference_edges()
-> std::result::Result<(), Box<dyn Error>> {
    run_with_rust_analyzer(async {
        let repo_root = repo_root()?;
        let provider = RustAnalyzerProvider::new();
        let request = reference_request(&provider, &repo_root)?;
        let workspace_root_uri = file_uri(&request.workspace_root)?;
        let extraction = provider.extract_rust_references(request).await?;
        assert!(!extraction.references.is_empty());

        let db_path = temp_db_path()?;
        let writer = WriteManager::start(&db_path).await?;
        writer.migrate().await?;
        ExtractionPersister
            .persist_document_symbol_batch(
                &writer,
                &workspace_root_uri,
                &extraction.document_symbols,
            )
            .await?;
        ExtractionPersister
            .persist_reference_batch(&writer, &workspace_root_uri, &extraction)
            .await?;

        let mut second_extraction = extraction.clone();
        second_extraction.references.clear();
        second_extraction.summary.reference_edges = 0;
        second_extraction.summary.reference_occurrences = 0;
        ExtractionPersister
            .persist_reference_batch(&writer, &workspace_root_uri, &second_extraction)
            .await?;
        writer.shutdown().await?;

        let pool = sqlite_pool(&db_path).await?;
        let stale_reference_edges: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM edges WHERE relation = 'references' AND valid_to_run_id IS NOT NULL",
    )
    .fetch_one(&pool)
    .await?;

        assert_eq!(
            stale_reference_edges,
            i64::try_from(extraction.references.len())?
        );

        Ok(())
    })
}

fn reference_request(
    provider: &RustAnalyzerProvider,
    repo_root: &Path,
) -> std::result::Result<ReferenceBatchRequest, Box<dyn Error>> {
    let package_path = repo_root.join("crates/wip");
    let file_paths = provider.discover_rust_source_files(repo_root, &package_path)?;
    Ok(ReferenceBatchRequest {
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
        "poc-semanticgraph-reference-extract-{}-{stamp}.db",
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
