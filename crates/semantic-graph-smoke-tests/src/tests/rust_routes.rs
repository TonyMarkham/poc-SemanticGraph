use std::error::Error;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use semantic_graph_extract::document_symbols::paths::file_uri;
use semantic_graph_extract::model::{DocumentSymbolBatchRequest, ReferenceBatchRequest};
use semantic_graph_extract::persist::ExtractionPersister;
use semantic_graph_extract::providers::rust_analyzer::RustAnalyzerProvider;
use semantic_graph_store::GraphStore;
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
        let store = GraphStore::connect(&db_path).await?;
        store.migrate().await?;
        let workspace_root_uri = file_uri(&repo_root)?;
        let summary = ExtractionPersister
            .persist_document_symbol_batch(&store, &workspace_root_uri, &extraction)
            .await?;

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
        let store = GraphStore::connect(&db_path).await?;
        store.migrate().await?;
        let workspace_root_uri = file_uri(&repo_root)?;
        let summary = ExtractionPersister
            .persist_document_symbol_batch(&store, &workspace_root_uri, &extraction)
            .await?;

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
        let store = GraphStore::connect(&db_path).await?;
        store.migrate().await?;
        let workspace_root_uri = file_uri(&repo_root)?;
        let summary = ExtractionPersister
            .persist_reference_batch(&store, &workspace_root_uri, &extraction)
            .await?;

        assert_eq!(summary.files, file_count);
        assert_eq!(summary.reference_edges, extraction.summary.reference_edges);
        assert_eq!(
            summary.reference_occurrences,
            extraction.summary.reference_occurrences
        );
        assert_eq!(summary.routes_complete, file_count + 1);

        Ok::<(), Box<dyn Error>>(())
    })?;

    Ok(())
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
