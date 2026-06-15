use std::error::Error;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use lsp_types::DocumentSymbol;
use semantic_graph_db_manager::WriteManager;
use semantic_graph_extract::document_symbols::paths::file_uri;
use semantic_graph_extract::model::{
    CallBatchRequest, DocumentSymbolBatchRequest, ReferenceBatchRequest,
};
use semantic_graph_extract::persist::ExtractionPersister;
use semantic_graph_extract::providers::rust_analyzer::RustAnalyzerProvider;

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
    let store = WriteManager::start(&db_path).await?;
    store.migrate().await?;
    let workspace_root_uri = file_uri(&workspace_root)?;
    let summary = ExtractionPersister
        .persist_document_symbol_batch(&store, &workspace_root_uri, &extraction)
        .await?;
    store.shutdown().await?;
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
            file_paths: workspace_files.clone(),
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
    let workspace_store = WriteManager::start(&workspace_db_path).await?;
    workspace_store.migrate().await?;
    let workspace_summary = ExtractionPersister
        .persist_document_symbol_batch(&workspace_store, &workspace_root_uri, &workspace_extraction)
        .await?;
    workspace_store.shutdown().await?;
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

    let reference_extraction = provider
        .extract_rust_references(ReferenceBatchRequest {
            workspace_root: workspace_root.clone(),
            package_path: workspace_root.clone(),
            file_paths: workspace_files.clone(),
        })
        .await?;
    println!(
        "workspace.references.targets={}",
        reference_extraction.summary.targets_queried
    );
    println!(
        "workspace.references.edges={}",
        reference_extraction.summary.reference_edges
    );
    println!(
        "workspace.references.occurrences={}",
        reference_extraction.summary.reference_occurrences
    );
    println!(
        "workspace.references.file_fallbacks={}",
        reference_extraction.summary.file_fallbacks
    );
    println!(
        "workspace.references.skipped_external={}",
        reference_extraction.summary.skipped_external
    );
    ensure(
        reference_extraction.summary.targets_queried > 0,
        "workspace references queried at least one target",
    )?;
    ensure(
        reference_extraction.summary.reference_edges > 0,
        "workspace references produced canonical edges",
    )?;
    ensure(
        reference_extraction.summary.reference_occurrences > 0,
        "workspace references produced occurrences",
    )?;

    let reference_db_path = temp_db_path("workspace-references")?;
    let reference_store = WriteManager::start(&reference_db_path).await?;
    reference_store.migrate().await?;
    let reference_base_summary = ExtractionPersister
        .persist_document_symbol_batch(
            &reference_store,
            &workspace_root_uri,
            &reference_extraction.document_symbols,
        )
        .await?;
    let reference_summary = ExtractionPersister
        .persist_reference_batch(&reference_store, &workspace_root_uri, &reference_extraction)
        .await?;
    reference_store.shutdown().await?;
    println!(
        "workspace.references.persistence.db={}",
        reference_db_path.display()
    );
    println!(
        "workspace.references.base.files={}",
        reference_base_summary.files
    );
    println!(
        "workspace.references.base.nodes={}",
        reference_base_summary.nodes
    );
    println!(
        "workspace.references.base.contains_edges={}",
        reference_base_summary.edges
    );
    println!(
        "workspace.references.base.occurrences={}",
        reference_base_summary.occurrences
    );
    println!(
        "workspace.references.base.evidence={}",
        reference_base_summary.evidence
    );
    println!(
        "workspace.references.route.files={}",
        reference_summary.files
    );
    println!(
        "workspace.references.route.nodes={}",
        reference_summary.nodes
    );
    println!(
        "workspace.references.route.contains_edges={}",
        reference_summary
            .edges
            .saturating_sub(reference_summary.reference_edges)
    );
    println!(
        "workspace.references.route.references_edges={}",
        reference_summary.reference_edges
    );
    println!(
        "workspace.references.route.reference_occurrences={}",
        reference_summary.reference_occurrences
    );
    println!(
        "workspace.references.route.evidence={}",
        reference_summary.evidence
    );
    println!(
        "workspace.references.route.routes_complete={}",
        reference_summary.routes_complete
    );
    println!(
        "workspace.references.route.stale_nodes_closed={}",
        reference_summary.stale_nodes_closed
    );
    println!(
        "workspace.references.route.stale_edges_closed={}",
        reference_summary.stale_edges_closed
    );
    ensure(
        reference_base_summary.files == workspace_file_count,
        "reference base persistence wrote every discovered source file",
    )?;
    ensure(
        reference_summary.files == 0,
        "reference route persistence did not rewrite document-symbol files",
    )?;
    ensure(
        reference_summary.reference_edges == reference_extraction.summary.reference_edges,
        "reference persistence wrote every reference edge",
    )?;
    ensure(
        reference_summary.reference_occurrences
            == reference_extraction.summary.reference_occurrences,
        "reference persistence wrote every reference occurrence",
    )?;
    ensure(
        reference_summary.routes_complete == 1,
        "reference persistence completed only the reference route",
    )?;

    let call_extraction = provider
        .extract_rust_calls(CallBatchRequest {
            workspace_root: workspace_root.clone(),
            package_path: workspace_root.clone(),
            file_paths: workspace_files,
        })
        .await?;
    println!(
        "workspace.calls.callable_nodes={}",
        call_extraction.summary.callable_nodes
    );
    println!(
        "workspace.calls.edges={}",
        call_extraction.summary.call_edges
    );
    println!(
        "workspace.calls.occurrences={}",
        call_extraction.summary.call_occurrences
    );
    println!(
        "workspace.calls.skipped_external_targets={}",
        call_extraction.summary.skipped_external_targets
    );
    println!(
        "workspace.calls.skipped_unresolved_targets={}",
        call_extraction.summary.skipped_unresolved_targets
    );
    ensure(
        call_extraction.summary.callable_nodes > 0,
        "workspace calls queried at least one callable",
    )?;
    ensure(
        call_extraction.summary.call_edges > 0,
        "workspace calls produced canonical edges",
    )?;
    ensure(
        call_extraction.summary.call_occurrences > 0,
        "workspace calls produced occurrences",
    )?;

    let call_db_path = temp_db_path("workspace-calls")?;
    let call_store = WriteManager::start(&call_db_path).await?;
    call_store.migrate().await?;
    let call_base_summary = ExtractionPersister
        .persist_document_symbol_batch(
            &call_store,
            &workspace_root_uri,
            &call_extraction.document_symbols,
        )
        .await?;
    let call_summary = ExtractionPersister
        .persist_call_batch(&call_store, &workspace_root_uri, &call_extraction)
        .await?;
    call_store.shutdown().await?;
    println!("workspace.calls.persistence.db={}", call_db_path.display());
    println!("workspace.calls.base.files={}", call_base_summary.files);
    println!("workspace.calls.base.nodes={}", call_base_summary.nodes);
    println!(
        "workspace.calls.base.contains_edges={}",
        call_base_summary.edges
    );
    println!(
        "workspace.calls.base.occurrences={}",
        call_base_summary.occurrences
    );
    println!(
        "workspace.calls.base.evidence={}",
        call_base_summary.evidence
    );
    println!("workspace.calls.route.files={}", call_summary.files);
    println!("workspace.calls.route.nodes={}", call_summary.nodes);
    println!(
        "workspace.calls.route.contains_edges={}",
        call_summary
            .edges
            .saturating_sub(call_summary.reference_edges)
            .saturating_sub(call_summary.call_edges)
    );
    println!(
        "workspace.calls.route.calls_edges={}",
        call_summary.call_edges
    );
    println!(
        "workspace.calls.route.call_occurrences={}",
        call_summary.call_occurrences
    );
    println!("workspace.calls.route.evidence={}", call_summary.evidence);
    println!(
        "workspace.calls.route.routes_complete={}",
        call_summary.routes_complete
    );
    println!(
        "workspace.calls.route.stale_nodes_closed={}",
        call_summary.stale_nodes_closed
    );
    println!(
        "workspace.calls.route.stale_edges_closed={}",
        call_summary.stale_edges_closed
    );
    ensure(
        call_base_summary.files == workspace_file_count,
        "call base persistence wrote every discovered source file",
    )?;
    ensure(
        call_summary.files == 0,
        "call route persistence did not rewrite document-symbol files",
    )?;
    ensure(
        call_summary.call_edges == call_extraction.summary.call_edges,
        "call persistence wrote every call edge",
    )?;
    ensure(
        call_summary.call_occurrences == call_extraction.summary.call_occurrences,
        "call persistence wrote every call occurrence",
    )?;
    ensure(
        call_summary.routes_complete == 1,
        "call persistence completed only the calls route",
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
