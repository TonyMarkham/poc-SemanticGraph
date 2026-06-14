use semantic_graph_extract::{
    ExtractError, ExtractResult,
    document_symbols::paths::{
        file_uri, validate_document_symbol_batch_request, validate_document_symbol_request,
    },
    model::{DocumentSymbolBatchRequest, DocumentSymbolRequest},
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
    }

    Ok(())
}
