use semantic_graph_extract::{
    ExtractError, ExtractResult,
    document_symbols::paths::{
        file_uri, validate_document_symbol_batch_request, validate_document_symbol_request,
    },
    model::{
        CallBatchRequest, DocumentSymbolBatchRequest, DocumentSymbolRequest, ReferenceBatchRequest,
    },
    persist::ExtractionPersister,
    provider::DocumentSymbolProvider,
    providers::rust_analyzer::RustAnalyzerProvider,
};

use semantic_graph_store::GraphStore;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(about = "Language-server-backed semantic graph extraction prototype")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    #[command(name = "rust-document-symbols")]
    SingleFile {
        #[arg(long)]
        db: PathBuf,
        #[arg(long)]
        workspace_root: PathBuf,
        #[arg(long)]
        package_path: PathBuf,
        #[arg(long)]
        file: PathBuf,
    },
    #[command(name = "rust-crate-document-symbols")]
    CrateBatch {
        #[arg(long)]
        db: PathBuf,
        #[arg(long)]
        workspace_root: PathBuf,
        #[arg(long)]
        package_path: PathBuf,
    },
    #[command(name = "rust-workspace-document-symbols")]
    WorkspaceBatch {
        #[arg(long)]
        db: PathBuf,
        #[arg(long)]
        workspace_root: PathBuf,
    },
    #[command(name = "rust-workspace-references")]
    WorkspaceReferences {
        #[arg(long)]
        db: PathBuf,
        #[arg(long)]
        workspace_root: PathBuf,
    },
    #[command(name = "rust-workspace-calls")]
    WorkspaceCalls {
        #[arg(long)]
        db: PathBuf,
        #[arg(long)]
        workspace_root: PathBuf,
    },
    #[command(name = "rust-workspace-all")]
    WorkspaceAll {
        #[arg(long)]
        db: PathBuf,
        #[arg(long)]
        workspace_root: PathBuf,
    },
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

async fn run() -> ExtractResult<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::SingleFile {
            db,
            workspace_root,
            package_path,
            file,
        } => {
            let request = validate_document_symbol_request(DocumentSymbolRequest {
                workspace_root,
                package_path,
                file_path: file,
            })?;
            let workspace_root_uri = file_uri(&request.workspace_root)?;
            let store = GraphStore::connect(db)
                .await
                .map_err(ExtractError::storage)?;
            store.migrate().await.map_err(ExtractError::storage)?;

            let provider = RustAnalyzerProvider::new();
            let extraction = provider.extract_document_symbols(request).await?;
            let summary = ExtractionPersister
                .persist_document_symbols(&store, &workspace_root_uri, &extraction)
                .await?;

            println!(
                "workspace={} run={} files={} nodes={} edges={} occurrences={} evidence={}",
                summary.workspace_id,
                summary.run_id,
                summary.files,
                summary.nodes,
                summary.edges,
                summary.occurrences,
                summary.evidence
            );
        }
        Command::CrateBatch {
            db,
            workspace_root,
            package_path,
        } => {
            let provider = RustAnalyzerProvider::new();
            let file_paths = provider.discover_rust_source_files(&workspace_root, &package_path)?;
            let request = validate_document_symbol_batch_request(DocumentSymbolBatchRequest {
                workspace_root,
                package_path,
                file_paths,
            })?;
            let workspace_root_uri = file_uri(&request.workspace_root)?;
            let store = GraphStore::connect(db)
                .await
                .map_err(ExtractError::storage)?;
            store.migrate().await.map_err(ExtractError::storage)?;

            let extraction = provider.extract_document_symbol_batch(request).await?;
            let summary = ExtractionPersister
                .persist_document_symbol_batch(&store, &workspace_root_uri, &extraction)
                .await?;

            println!(
                "workspace={} run={} files={} nodes={} edges={} occurrences={} evidence={}",
                summary.workspace_id,
                summary.run_id,
                summary.files,
                summary.nodes,
                summary.edges,
                summary.occurrences,
                summary.evidence
            );
        }
        Command::WorkspaceBatch { db, workspace_root } => {
            let provider = RustAnalyzerProvider::new();
            let file_paths = provider.discover_rust_workspace_source_files(&workspace_root)?;
            let request = validate_document_symbol_batch_request(DocumentSymbolBatchRequest {
                package_path: workspace_root.clone(),
                workspace_root,
                file_paths,
            })?;
            let workspace_root_uri = file_uri(&request.workspace_root)?;
            let store = GraphStore::connect(db)
                .await
                .map_err(ExtractError::storage)?;
            store.migrate().await.map_err(ExtractError::storage)?;

            let extraction = provider.extract_document_symbol_batch(request).await?;
            let summary = ExtractionPersister
                .persist_document_symbol_batch(&store, &workspace_root_uri, &extraction)
                .await?;

            println!(
                "workspace={} run={} files={} nodes={} edges={} occurrences={} evidence={}",
                summary.workspace_id,
                summary.run_id,
                summary.files,
                summary.nodes,
                summary.edges,
                summary.occurrences,
                summary.evidence
            );
        }
        Command::WorkspaceReferences { db, workspace_root } => {
            let provider = RustAnalyzerProvider::new();
            let file_paths = provider.discover_rust_workspace_source_files(&workspace_root)?;
            let document_request =
                validate_document_symbol_batch_request(DocumentSymbolBatchRequest {
                    package_path: workspace_root.clone(),
                    workspace_root,
                    file_paths,
                })?;
            let workspace_root_uri = file_uri(&document_request.workspace_root)?;
            let store = GraphStore::connect(db)
                .await
                .map_err(ExtractError::storage)?;
            store.migrate().await.map_err(ExtractError::storage)?;

            let extraction = provider
                .extract_rust_references(ReferenceBatchRequest {
                    workspace_root: document_request.workspace_root,
                    package_path: document_request.package_path,
                    file_paths: document_request.file_paths,
                })
                .await?;
            let summary = ExtractionPersister
                .persist_reference_batch(&store, &workspace_root_uri, &extraction)
                .await?;

            println!(
                "workspace={} run={} targets={} references_edges={} reference_occurrences={} evidence={} stale_edges_closed={}",
                summary.workspace_id,
                summary.run_id,
                extraction.summary.targets_queried,
                summary.reference_edges,
                summary.reference_occurrences,
                summary.evidence,
                summary.stale_edges_closed
            );
        }
        Command::WorkspaceCalls { db, workspace_root } => {
            let provider = RustAnalyzerProvider::new();
            let file_paths = provider.discover_rust_workspace_source_files(&workspace_root)?;
            let document_request =
                validate_document_symbol_batch_request(DocumentSymbolBatchRequest {
                    package_path: workspace_root.clone(),
                    workspace_root,
                    file_paths,
                })?;
            let workspace_root_uri = file_uri(&document_request.workspace_root)?;
            let store = GraphStore::connect(db)
                .await
                .map_err(ExtractError::storage)?;
            store.migrate().await.map_err(ExtractError::storage)?;

            let extraction = provider
                .extract_rust_calls(CallBatchRequest {
                    workspace_root: document_request.workspace_root,
                    package_path: document_request.package_path,
                    file_paths: document_request.file_paths,
                })
                .await?;
            let summary = ExtractionPersister
                .persist_call_batch(&store, &workspace_root_uri, &extraction)
                .await?;

            println!(
                "workspace={} run={} callable_nodes={} calls_edges={} call_occurrences={} evidence={} skipped_external_targets={} skipped_unresolved_targets={} stale_edges_closed={}",
                summary.workspace_id,
                summary.run_id,
                extraction.summary.callable_nodes,
                summary.call_edges,
                summary.call_occurrences,
                summary.evidence,
                extraction.summary.skipped_external_targets,
                extraction.summary.skipped_unresolved_targets,
                summary.stale_edges_closed
            );
        }
        Command::WorkspaceAll { db, workspace_root } => {
            let provider = RustAnalyzerProvider::new();
            let file_paths = provider.discover_rust_workspace_source_files(&workspace_root)?;
            let document_request =
                validate_document_symbol_batch_request(DocumentSymbolBatchRequest {
                    package_path: workspace_root.clone(),
                    workspace_root,
                    file_paths,
                })?;
            let workspace_root_uri = file_uri(&document_request.workspace_root)?;
            let store = GraphStore::connect(db)
                .await
                .map_err(ExtractError::storage)?;
            store.migrate().await.map_err(ExtractError::storage)?;

            let reference_extraction = provider
                .extract_rust_references(ReferenceBatchRequest {
                    workspace_root: document_request.workspace_root.clone(),
                    package_path: document_request.package_path.clone(),
                    file_paths: document_request.file_paths.clone(),
                })
                .await?;
            let call_extraction = provider
                .extract_rust_calls(CallBatchRequest {
                    workspace_root: document_request.workspace_root,
                    package_path: document_request.package_path,
                    file_paths: document_request.file_paths,
                })
                .await?;
            let document_summary = ExtractionPersister
                .persist_document_symbol_batch(
                    &store,
                    &workspace_root_uri,
                    &reference_extraction.document_symbols,
                )
                .await?;
            let reference_summary = ExtractionPersister
                .persist_reference_batch(&store, &workspace_root_uri, &reference_extraction)
                .await?;
            let call_summary = ExtractionPersister
                .persist_call_batch(&store, &workspace_root_uri, &call_extraction)
                .await?;

            println!(
                "workspace={} document_run={} reference_run={} call_run={} files={} nodes={} contains_edges={} references_edges={} reference_occurrences={} calls_edges={} call_occurrences={} evidence={} routes_complete={} stale_nodes_closed={} stale_edges_closed={}",
                document_summary.workspace_id,
                document_summary.run_id,
                reference_summary.run_id,
                call_summary.run_id,
                document_summary.files,
                document_summary.nodes,
                document_summary.edges,
                reference_summary.reference_edges,
                reference_summary.reference_occurrences,
                call_summary.call_edges,
                call_summary.call_occurrences,
                document_summary.evidence + reference_summary.evidence + call_summary.evidence,
                document_summary.routes_complete
                    + reference_summary.routes_complete
                    + call_summary.routes_complete,
                document_summary.stale_nodes_closed,
                document_summary.stale_edges_closed
                    + reference_summary.stale_edges_closed
                    + call_summary.stale_edges_closed
            );
        }
    }

    Ok(())
}
