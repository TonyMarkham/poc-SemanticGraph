use crate::{
    model::DocumentSymbolBatchRequest,
    providers::rust_analyzer::RustAnalyzerProvider,
    workspace_extraction::{ThreadedWorkspaceExtractionConfig, ThreadedWorkspaceExtractionRunner},
};

use semantic_graph_db_manager::WriteManager;
use sqlx::SqlitePool;
use std::{
    env,
    error::Error,
    future::Future,
    io,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::runtime::Builder;

static RUST_ANALYZER_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn threaded_workspace_extraction_streams_relation_writes() -> std::result::Result<(), Box<dyn Error>>
{
    run_with_rust_analyzer(async {
        let repo_root = repo_root()?;
        let provider = RustAnalyzerProvider::new();
        let request = workspace_extraction_request(&provider, &repo_root)?;
        let db_path = temp_db_path()?;
        let writer = WriteManager::start(&db_path).await?;
        writer.migrate().await?;

        let summary = ThreadedWorkspaceExtractionRunner::run(
            &writer,
            &provider,
            request,
            ThreadedWorkspaceExtractionConfig::new(1, 1, 1, 0, 0),
        )
        .await?;
        writer.shutdown().await?;

        assert!(summary.document_summary.files > 0);
        assert!(summary.reference_summary.reference_edges > 0);
        assert!(summary.call_summary.call_edges > 0);
        assert_eq!(summary.document_summary.routes_complete, 4);
        assert_eq!(summary.reference_summary.routes_complete, 1);
        assert_eq!(summary.call_summary.routes_complete, 1);

        let pool = sqlite_pool(&db_path).await?;
        let complete_workspace_routes: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM extraction_route_status
            WHERE scope = 'workspace'
              AND route IN ('rust.references', 'rust.calls')
              AND last_status = 'complete'
              AND json_extract(diagnostics_json, '$.actual_execution_mode') = 'file_grained_analysis_worker_pool'
            "#,
        )
        .fetch_one(&pool)
        .await?;
        let reference_edges: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM edges WHERE relation = 'references'")
                .fetch_one(&pool)
                .await?;
        let call_edges: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM edges WHERE relation = 'calls'")
                .fetch_one(&pool)
                .await?;

        assert_eq!(complete_workspace_routes, 2);
        assert_eq!(
            reference_edges,
            i64::try_from(summary.reference_summary.reference_edges)?
        );
        assert_eq!(call_edges, i64::try_from(summary.call_summary.call_edges)?);
        Ok(())
    })
}

fn workspace_extraction_request(
    provider: &RustAnalyzerProvider,
    repo_root: &Path,
) -> std::result::Result<DocumentSymbolBatchRequest, Box<dyn Error>> {
    let package_path = repo_root.join("crates/wip");
    let file_paths = provider.discover_rust_source_files(repo_root, &package_path)?;
    Ok(DocumentSymbolBatchRequest {
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
        "poc-semanticgraph-threaded-workspace-extraction-{}-{stamp}.db",
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
