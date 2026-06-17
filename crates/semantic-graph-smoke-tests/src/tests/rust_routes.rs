use std::error::Error;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use semantic_graph_db_manager::WriteManager;
use semantic_graph_extract::document_symbols::paths::file_uri;
use semantic_graph_extract::model::{
    CallBatchRequest, DocumentSymbolBatchRequest, ReferenceBatchRequest,
};
use semantic_graph_extract::persist::ExtractionPersister;
use semantic_graph_extract::providers::rust_analyzer::RustAnalyzerProvider;
use semantic_graph_extract::workspace_extraction::{
    SharedWorkspaceExtractionRunner, ThreadedWorkspaceExtractionConfig,
    ThreadedWorkspaceExtractionRunner, WorkspaceExtractionRoutes, WorkspaceExtractionSummary,
};
use tokio::runtime::Builder;

static WORKSPACE_LOAD_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn facade_routes_discover_wip_sources_and_symbols() -> Result<(), Box<dyn Error>> {
    let _guard = workspace_load_guard()?;
    let repo_root = repo_root()?;
    let package_path = repo_root.join("crates/wip");
    let model = rust_analyzer_lib::load_workspace(&repo_root)?;
    let files = rust_analyzer_lib::package_source_files(&model, &package_path);
    let relative_files = relative_paths(&repo_root, &files)?;

    assert_eq!(
        relative_files,
        vec![
            "crates/wip/src/lib.rs".to_string(),
            "crates/wip/src/models.rs".to_string(),
            "crates/wip/src/pipeline.rs".to_string(),
            "crates/wip/src/tests/mod.rs".to_string(),
        ]
    );

    let symbol_batches = rust_analyzer_lib::document_symbols_for_files(&repo_root, &files)?;
    assert_eq!(symbol_batches.len(), 4);
    assert!(
        symbol_batches
            .iter()
            .flat_map(|(_path, symbols)| symbols)
            .any(|symbol| symbol.name == "Widget")
    );

    Ok(())
}

#[test]
fn extractor_route_persists_wip_batch_without_binary() -> Result<(), Box<dyn Error>> {
    let _guard = workspace_load_guard()?;
    let repo_root = repo_root()?;
    let package_path = repo_root.join("crates/wip");
    let provider = RustAnalyzerProvider::new();
    let file_paths = provider.discover_rust_source_files(&repo_root, &package_path)?;

    assert_eq!(
        relative_paths(&repo_root, &file_paths)?,
        vec![
            "crates/wip/src/lib.rs".to_string(),
            "crates/wip/src/models.rs".to_string(),
            "crates/wip/src/pipeline.rs".to_string(),
            "crates/wip/src/tests/mod.rs".to_string(),
        ]
    );

    let runtime = Builder::new_current_thread().enable_all().build()?;
    runtime.block_on(async {
        let extraction = provider
            .extract_document_symbol_batch(DocumentSymbolBatchRequest {
                workspace_root: repo_root.clone(),
                package_path,
                file_paths,
            })
            .await?;

        assert_eq!(extraction.extractions.len(), 4);
        assert!(
            extraction
                .provider_version
                .as_deref()
                .is_some_and(|version| version.starts_with("rust-analyzer-lib "))
        );

        let db_path = temp_db_path("wip-batch")?;
        let store = WriteManager::start(&db_path).await?;
        store.migrate().await?;
        let workspace_root_uri = file_uri(&repo_root)?;
        let summary = ExtractionPersister
            .persist_document_symbol_batch(&store, &workspace_root_uri, &extraction)
            .await?;
        store.shutdown().await?;

        assert_eq!(summary.files, 4);
        assert!(summary.nodes > 3);
        assert!(summary.edges > 0);
        assert!(summary.occurrences > 0);
        assert!(summary.evidence > 0);

        Ok::<(), Box<dyn Error>>(())
    })?;

    Ok(())
}

#[test]
fn workspace_route_persists_workspace_batch_without_binary() -> Result<(), Box<dyn Error>> {
    let _guard = workspace_load_guard()?;
    let repo_root = repo_root()?;
    let provider = RustAnalyzerProvider::new();
    let files = provider.discover_rust_workspace_source_files(&repo_root)?;
    let relative_files = relative_paths(&repo_root, &files)?;
    let file_count = files.len();

    assert!(
        relative_files
            .iter()
            .any(|path| path == "crates/wip/src/lib.rs")
    );
    assert!(
        relative_files
            .iter()
            .all(|path| !path.starts_with("submodules/"))
    );
    assert!(file_count > 4);

    let runtime = Builder::new_current_thread().enable_all().build()?;
    runtime.block_on(async {
        let extraction = provider
            .extract_document_symbol_batch(DocumentSymbolBatchRequest {
                workspace_root: repo_root.clone(),
                package_path: repo_root.clone(),
                file_paths: files,
            })
            .await?;

        assert_eq!(extraction.extractions.len(), file_count);
        assert!(
            extraction
                .provider_version
                .as_deref()
                .is_some_and(|version| version.starts_with("rust-analyzer-lib "))
        );
        assert!(
            extraction
                .extractions
                .iter()
                .map(|extraction| extraction.symbols.len())
                .sum::<usize>()
                > 0
        );

        let db_path = temp_db_path("workspace-batch")?;
        let store = WriteManager::start(&db_path).await?;
        store.migrate().await?;
        let workspace_root_uri = file_uri(&repo_root)?;
        let summary = ExtractionPersister
            .persist_document_symbol_batch(&store, &workspace_root_uri, &extraction)
            .await?;
        store.shutdown().await?;

        assert_eq!(summary.files, file_count);
        assert!(summary.nodes > summary.files);
        assert!(summary.edges > 0);
        assert!(summary.occurrences > 0);
        assert!(summary.evidence > 0);

        Ok::<(), Box<dyn Error>>(())
    })?;

    Ok(())
}

#[test]
#[ignore = "full workspace rust-analyzer references smoke; run explicitly for route confidence"]
fn workspace_route_persists_workspace_references_without_binary() -> Result<(), Box<dyn Error>> {
    let _guard = workspace_load_guard()?;
    let repo_root = repo_root()?;
    let provider = RustAnalyzerProvider::new();
    let files = provider.discover_rust_workspace_source_files(&repo_root)?;
    let file_count = files.len();

    let runtime = Builder::new_current_thread().enable_all().build()?;
    runtime.block_on(async {
        let extraction = provider
            .extract_rust_references(ReferenceBatchRequest {
                workspace_root: repo_root.clone(),
                package_path: repo_root.clone(),
                file_paths: files,
            })
            .await?;

        assert_eq!(extraction.document_symbols.extractions.len(), file_count);
        assert!(extraction.summary.targets_queried > 0);
        assert!(extraction.summary.reference_edges > 0);
        assert!(extraction.summary.reference_occurrences > 0);

        let db_path = temp_db_path("workspace-references")?;
        let store = WriteManager::start(&db_path).await?;
        store.migrate().await?;
        let workspace_root_uri = file_uri(&repo_root)?;
        ExtractionPersister
            .persist_document_symbol_batch(
                &store,
                &workspace_root_uri,
                &extraction.document_symbols,
            )
            .await?;
        let summary = ExtractionPersister
            .persist_reference_batch(&store, &workspace_root_uri, &extraction)
            .await?;
        store.shutdown().await?;

        assert_eq!(summary.files, 0);
        assert_eq!(summary.reference_edges, extraction.summary.reference_edges);
        assert_eq!(
            summary.reference_occurrences,
            extraction.summary.reference_occurrences
        );
        assert_eq!(summary.routes_complete, 1);

        Ok::<(), Box<dyn Error>>(())
    })?;

    Ok(())
}

#[test]
#[ignore = "full workspace rust-analyzer calls smoke; run explicitly for route confidence"]
fn workspace_route_persists_workspace_calls_without_binary() -> Result<(), Box<dyn Error>> {
    let _guard = workspace_load_guard()?;
    let repo_root = repo_root()?;
    let provider = RustAnalyzerProvider::new();
    let files = provider.discover_rust_workspace_source_files(&repo_root)?;
    let file_count = files.len();

    let runtime = Builder::new_current_thread().enable_all().build()?;
    runtime.block_on(async {
        let extraction = provider
            .extract_rust_calls(CallBatchRequest {
                workspace_root: repo_root.clone(),
                package_path: repo_root.clone(),
                file_paths: files,
            })
            .await?;

        assert_eq!(extraction.document_symbols.extractions.len(), file_count);
        assert!(extraction.summary.callable_nodes > 0);
        assert!(extraction.summary.call_edges > 0);
        assert!(extraction.summary.call_occurrences > 0);

        let db_path = temp_db_path("workspace-calls")?;
        let store = WriteManager::start(&db_path).await?;
        store.migrate().await?;
        let workspace_root_uri = file_uri(&repo_root)?;
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
        store.shutdown().await?;

        assert_eq!(summary.files, 0);
        assert_eq!(summary.call_edges, extraction.summary.call_edges);
        assert_eq!(
            summary.call_occurrences,
            extraction.summary.call_occurrences
        );
        assert_eq!(summary.routes_complete, 1);

        Ok::<(), Box<dyn Error>>(())
    })?;

    Ok(())
}

#[test]
#[ignore = "compares threaded and shared rust workspace counts on crates/wip"]
fn workspace_shared_matches_threaded_wip_counts() -> Result<(), Box<dyn Error>> {
    let _guard = workspace_load_guard()?;
    let repo_root = repo_root()?;
    let package_path = repo_root.join("crates/wip");
    let provider = RustAnalyzerProvider::new();
    let files = provider.discover_rust_source_files(&repo_root, &package_path)?;
    let threaded_db_path = temp_db_path("workspace-shared-threaded")?;
    let shared_db_path = temp_db_path("workspace-shared-optimized")?;
    let routes = WorkspaceExtractionRoutes::all();
    let config = ThreadedWorkspaceExtractionConfig::with_routes(2, 2, 2, 0, 0, routes);
    let request = DocumentSymbolBatchRequest {
        workspace_root: repo_root,
        package_path,
        file_paths: files,
    };

    let runtime = Builder::new_current_thread().enable_all().build()?;
    runtime.block_on(async {
        let threaded_store = WriteManager::start(&threaded_db_path).await?;
        threaded_store.migrate().await?;
        let threaded_summary = ThreadedWorkspaceExtractionRunner::run(
            &threaded_store,
            &provider,
            request.clone(),
            config.clone(),
        )
        .await?;
        threaded_store.shutdown().await?;

        let shared_store = WriteManager::start(&shared_db_path).await?;
        shared_store.migrate().await?;
        let shared_summary =
            SharedWorkspaceExtractionRunner::run(&shared_store, &provider, request, config, false)
                .await?;
        shared_store.shutdown().await?;

        print_shared_comparison(
            &threaded_db_path,
            &shared_db_path,
            &threaded_summary,
            &shared_summary,
        );
        assert_shared_matches_threaded(&threaded_summary, &shared_summary);

        Ok::<(), Box<dyn Error>>(())
    })?;

    Ok(())
}

fn print_shared_comparison(
    threaded_db_path: &Path,
    shared_db_path: &Path,
    threaded: &WorkspaceExtractionSummary,
    shared: &WorkspaceExtractionSummary,
) {
    println!(
        "workspace.shared_vs_threaded.threaded.db={}",
        threaded_db_path.display()
    );
    println!(
        "workspace.shared_vs_threaded.shared.db={}",
        shared_db_path.display()
    );
    println!(
        "workspace.shared_vs_threaded.threaded.files={}",
        threaded.document_summary.files
    );
    println!(
        "workspace.shared_vs_threaded.shared.files={}",
        shared.document_summary.files
    );
    println!(
        "workspace.shared_vs_threaded.threaded.nodes={}",
        threaded.document_summary.nodes
    );
    println!(
        "workspace.shared_vs_threaded.shared.nodes={}",
        shared.document_summary.nodes
    );
    println!(
        "workspace.shared_vs_threaded.threaded.contains_edges={}",
        threaded.document_summary.edges
    );
    println!(
        "workspace.shared_vs_threaded.shared.contains_edges={}",
        shared.document_summary.edges
    );
    println!(
        "workspace.shared_vs_threaded.threaded.reference_edges={}",
        threaded.reference_summary.reference_edges
    );
    println!(
        "workspace.shared_vs_threaded.shared.reference_edges={}",
        shared.reference_summary.reference_edges
    );
    println!(
        "workspace.shared_vs_threaded.threaded.reference_occurrences={}",
        threaded.reference_summary.reference_occurrences
    );
    println!(
        "workspace.shared_vs_threaded.shared.reference_occurrences={}",
        shared.reference_summary.reference_occurrences
    );
    println!(
        "workspace.shared_vs_threaded.threaded.call_edges={}",
        threaded.call_summary.call_edges
    );
    println!(
        "workspace.shared_vs_threaded.shared.call_edges={}",
        shared.call_summary.call_edges
    );
    println!(
        "workspace.shared_vs_threaded.threaded.call_occurrences={}",
        threaded.call_summary.call_occurrences
    );
    println!(
        "workspace.shared_vs_threaded.shared.call_occurrences={}",
        shared.call_summary.call_occurrences
    );
    println!(
        "workspace.shared_vs_threaded.threaded.routes_complete={}",
        threaded.document_summary.routes_complete
            + threaded.reference_summary.routes_complete
            + threaded.call_summary.routes_complete
    );
    println!(
        "workspace.shared_vs_threaded.shared.routes_complete={}",
        shared.document_summary.routes_complete
            + shared.reference_summary.routes_complete
            + shared.call_summary.routes_complete
    );
    println!(
        "workspace.shared_vs_threaded.threaded.stale_edges_closed={}",
        threaded.document_summary.stale_edges_closed
            + threaded.reference_summary.stale_edges_closed
            + threaded.call_summary.stale_edges_closed
    );
    println!(
        "workspace.shared_vs_threaded.shared.stale_edges_closed={}",
        shared.document_summary.stale_edges_closed
            + shared.reference_summary.stale_edges_closed
            + shared.call_summary.stale_edges_closed
    );
    for line in threaded.benchmark.lines() {
        println!("workspace.shared_vs_threaded.threaded.{line}");
    }
    for line in shared.benchmark.lines() {
        println!("workspace.shared_vs_threaded.shared.{line}");
    }
}

fn assert_shared_matches_threaded(
    threaded: &WorkspaceExtractionSummary,
    shared: &WorkspaceExtractionSummary,
) {
    assert_eq!(
        shared.document_summary.files,
        threaded.document_summary.files
    );
    assert_eq!(
        shared.document_summary.nodes,
        threaded.document_summary.nodes
    );
    assert_eq!(
        shared.document_summary.edges,
        threaded.document_summary.edges
    );
    assert_eq!(
        shared.document_summary.occurrences,
        threaded.document_summary.occurrences
    );
    assert_eq!(
        shared.document_summary.evidence,
        threaded.document_summary.evidence
    );
    assert_eq!(
        shared.reference_summary.reference_edges,
        threaded.reference_summary.reference_edges
    );
    assert_eq!(
        shared.reference_summary.reference_occurrences,
        threaded.reference_summary.reference_occurrences
    );
    assert_eq!(
        shared.call_summary.call_edges,
        threaded.call_summary.call_edges
    );
    assert_eq!(
        shared.call_summary.call_occurrences,
        threaded.call_summary.call_occurrences
    );
    assert_eq!(
        shared.document_summary.routes_complete
            + shared.reference_summary.routes_complete
            + shared.call_summary.routes_complete,
        threaded.document_summary.routes_complete
            + threaded.reference_summary.routes_complete
            + threaded.call_summary.routes_complete
    );
}

fn repo_root() -> Result<PathBuf, Box<dyn Error>> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let crates_dir = manifest_dir
        .parent()
        .ok_or_else(|| io::Error::other("smoke-test manifest dir has no parent"))?;
    let repo_root = crates_dir
        .parent()
        .ok_or_else(|| io::Error::other("crates directory has no parent"))?;

    Ok(repo_root.to_path_buf())
}

fn temp_db_path(name: &str) -> Result<PathBuf, Box<dyn Error>> {
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    Ok(std::env::temp_dir().join(format!(
        "semantic-graph-smoke-{name}-{}-{stamp}.db",
        std::process::id()
    )))
}

fn workspace_load_guard() -> Result<std::sync::MutexGuard<'static, ()>, Box<dyn Error>> {
    WORKSPACE_LOAD_LOCK
        .lock()
        .map_err(|_| io::Error::other("workspace load smoke-test mutex was poisoned").into())
}

fn relative_paths(root: &Path, paths: &[PathBuf]) -> Result<Vec<String>, Box<dyn Error>> {
    paths.iter().map(|path| relative_path(root, path)).collect()
}

fn relative_path(root: &Path, path: &Path) -> Result<String, Box<dyn Error>> {
    let relative = path.strip_prefix(root)?;
    Ok(relative.to_string_lossy().replace('\\', "/"))
}
