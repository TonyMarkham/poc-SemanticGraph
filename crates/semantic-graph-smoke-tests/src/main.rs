use std::error::Error;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use lsp_types::DocumentSymbol;
use semantic_graph_extract::document_symbols::paths::file_uri;
use semantic_graph_extract::model::DocumentSymbolBatchRequest;
use semantic_graph_extract::persist::ExtractionPersister;
use semantic_graph_extract::providers::rust_analyzer::RustAnalyzerProvider;
use semantic_graph_store::GraphStore;

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("FAIL {error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn Error>> {
    let workspace_root = std::env::current_dir()?;
    let package_path = workspace_root.join("crates/wip");
    let provider = RustAnalyzerProvider::new();

    println!("semantic-graph rust route smoke");
    println!("workspace_root={}", workspace_root.display());
    println!("package_path={}", package_path.display());

    let model = rust_analyzer_lib::load_workspace(&workspace_root)?;
    let facade_package_files = rust_analyzer_lib::package_source_files(&model, &package_path);
    let facade_relative_files = relative_paths(&workspace_root, &facade_package_files)?;
    println!(
        "facade.package_source_files.count={}",
        facade_package_files.len()
    );
    for path in &facade_relative_files {
        println!("facade.package_source_files.file={path}");
    }
    ensure_wip_files(&facade_relative_files)?;

    let facade_symbols =
        rust_analyzer_lib::document_symbols_for_files(&workspace_root, &facade_package_files)?;
    let facade_symbol_count: usize = facade_symbols
        .iter()
        .map(|(_path, symbols)| count_symbols(symbols))
        .sum();
    println!("facade.document_symbols.files={}", facade_symbols.len());
    println!("facade.document_symbols.symbols={facade_symbol_count}");
    ensure(
        facade_symbols.len() == 4,
        "facade returned four symbol files",
    )?;
    ensure(
        facade_symbol_count > 0,
        "facade returned at least one document symbol",
    )?;

    let extractor_files = provider.discover_rust_source_files(&workspace_root, &package_path)?;
    let extractor_relative_files = relative_paths(&workspace_root, &extractor_files)?;
    println!("extractor.discovery.count={}", extractor_files.len());
    for path in &extractor_relative_files {
        println!("extractor.discovery.file={path}");
    }
    ensure_wip_files(&extractor_relative_files)?;

    let extraction = provider
        .extract_document_symbol_batch(DocumentSymbolBatchRequest {
            workspace_root: workspace_root.clone(),
            package_path: package_path.clone(),
            file_paths: extractor_files,
        })
        .await?;
    let extracted_symbol_count: usize = extraction
        .extractions
        .iter()
        .map(|extraction| extraction.symbols.len())
        .sum();
    println!("extractor.batch.files={}", extraction.extractions.len());
    println!("extractor.batch.symbols={extracted_symbol_count}");
    println!(
        "extractor.provider_version={}",
        extraction.provider_version.as_deref().unwrap_or("<none>")
    );
    ensure(
        extraction.extractions.len() == 4,
        "extractor returned four file extractions",
    )?;
    ensure(
        extracted_symbol_count > 0,
        "extractor returned at least one symbol",
    )?;

    let db_path = temp_db_path("crate")?;
    let store = GraphStore::connect(&db_path).await?;
    store.migrate().await?;
    let workspace_root_uri = file_uri(&workspace_root)?;
    let summary = ExtractionPersister
        .persist_document_symbol_batch(&store, &workspace_root_uri, &extraction)
        .await?;
    println!("crate.persistence.db={}", db_path.display());
    println!("crate.persistence.files={}", summary.files);
    println!("crate.persistence.nodes={}", summary.nodes);
    println!("crate.persistence.edges={}", summary.edges);
    println!("crate.persistence.occurrences={}", summary.occurrences);
    println!("crate.persistence.evidence={}", summary.evidence);
    ensure(summary.files == 4, "persistence wrote four files")?;
    ensure(summary.nodes > 3, "persistence wrote symbol nodes")?;
    ensure(summary.edges > 0, "persistence wrote contains edges")?;
    ensure(summary.occurrences > 0, "persistence wrote occurrences")?;
    ensure(summary.evidence > 0, "persistence wrote edge evidence")?;

    let workspace_files = provider.discover_rust_workspace_source_files(&workspace_root)?;
    let workspace_file_count = workspace_files.len();
    let workspace_relative_files = relative_paths(&workspace_root, &workspace_files)?;
    let submodule_file_count = workspace_relative_files
        .iter()
        .filter(|path| path.starts_with("submodules/"))
        .count();
    println!("workspace.discovery.count={workspace_file_count}");
    println!("workspace.discovery.submodule_files={submodule_file_count}");
    for path in &workspace_relative_files {
        println!("workspace.discovery.file={path}");
    }
    ensure(
        submodule_file_count == 0,
        "workspace discovery excluded submodules",
    )?;
    ensure(
        workspace_file_count > extraction.extractions.len(),
        "workspace discovery covered more than the WIP crate",
    )?;

    let workspace_extraction = provider
        .extract_document_symbol_batch(DocumentSymbolBatchRequest {
            workspace_root: workspace_root.clone(),
            package_path: workspace_root.clone(),
            file_paths: workspace_files,
        })
        .await?;
    let workspace_symbol_count: usize = workspace_extraction
        .extractions
        .iter()
        .map(|extraction| extraction.symbols.len())
        .sum();
    println!(
        "workspace.batch.files={}",
        workspace_extraction.extractions.len()
    );
    println!("workspace.batch.symbols={workspace_symbol_count}");
    println!(
        "workspace.provider_version={}",
        workspace_extraction
            .provider_version
            .as_deref()
            .unwrap_or("<none>")
    );
    ensure(
        workspace_extraction.extractions.len() == workspace_file_count,
        "workspace route extracted every discovered source file",
    )?;
    ensure(
        workspace_symbol_count > extracted_symbol_count,
        "workspace route extracted more symbols than the WIP crate",
    )?;

    let workspace_db_path = temp_db_path("workspace")?;
    let workspace_store = GraphStore::connect(&workspace_db_path).await?;
    workspace_store.migrate().await?;
    let workspace_summary = ExtractionPersister
        .persist_document_symbol_batch(&workspace_store, &workspace_root_uri, &workspace_extraction)
        .await?;
    println!("workspace.persistence.db={}", workspace_db_path.display());
    println!("workspace.persistence.files={}", workspace_summary.files);
    println!("workspace.persistence.nodes={}", workspace_summary.nodes);
    println!("workspace.persistence.edges={}", workspace_summary.edges);
    println!(
        "workspace.persistence.occurrences={}",
        workspace_summary.occurrences
    );
    println!(
        "workspace.persistence.evidence={}",
        workspace_summary.evidence
    );
    ensure(
        workspace_summary.files == workspace_file_count,
        "workspace persistence wrote every discovered source file",
    )?;
    ensure(
        workspace_summary.files > summary.files,
        "workspace persistence wrote more files than the WIP crate",
    )?;
    ensure(
        workspace_summary.nodes > summary.nodes,
        "workspace persistence wrote more nodes than the WIP crate",
    )?;
    ensure(
        workspace_summary.edges > summary.edges,
        "workspace persistence wrote more edges than the WIP crate",
    )?;
    ensure(
        workspace_summary.occurrences > summary.occurrences,
        "workspace persistence wrote more occurrences than the WIP crate",
    )?;
    ensure(
        workspace_summary.evidence > summary.evidence,
        "workspace persistence wrote more edge evidence than the WIP crate",
    )?;

    println!("PASS semantic-graph rust route smoke");
    Ok(())
}

fn ensure(condition: bool, message: &'static str) -> Result<(), Box<dyn Error>> {
    if condition {
        Ok(())
    } else {
        Err(io::Error::other(message).into())
    }
}

fn ensure_wip_files(paths: &[String]) -> Result<(), Box<dyn Error>> {
    ensure(
        paths
            == [
                "crates/wip/src/lib.rs",
                "crates/wip/src/models.rs",
                "crates/wip/src/pipeline.rs",
                "crates/wip/src/tests/mod.rs",
            ],
        "WIP source files matched expected module files",
    )
}

fn count_symbols(symbols: &[DocumentSymbol]) -> usize {
    symbols
        .iter()
        .map(|symbol| 1 + symbol.children.as_deref().map(count_symbols).unwrap_or(0))
        .sum()
}

fn temp_db_path(name: &str) -> Result<PathBuf, Box<dyn Error>> {
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    Ok(std::env::temp_dir().join(format!(
        "semantic-graph-rust-smoke-{name}-{}-{stamp}.db",
        std::process::id()
    )))
}

fn relative_paths(root: &Path, paths: &[PathBuf]) -> Result<Vec<String>, Box<dyn Error>> {
    paths.iter().map(|path| relative_path(root, path)).collect()
}

fn relative_path(root: &Path, path: &Path) -> Result<String, Box<dyn Error>> {
    let relative = path.strip_prefix(root)?;
    Ok(relative.to_string_lossy().replace('\\', "/"))
}
