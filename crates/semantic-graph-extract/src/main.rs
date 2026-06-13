use std::path::PathBuf;

use clap::{Parser, Subcommand};
use semantic_graph_extract::Result;
use semantic_graph_extract::document_symbols::paths::{file_uri, validate_document_symbol_request};
use semantic_graph_extract::model::DocumentSymbolRequest;
use semantic_graph_extract::persist::ExtractionPersister;
use semantic_graph_extract::provider::DocumentSymbolProvider;
use semantic_graph_extract::providers::rust_analyzer::RustAnalyzerProvider;
use semantic_graph_store::GraphStore;

#[derive(Debug, Parser)]
#[command(about = "Language-server-backed semantic graph extraction prototype")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    RustDocumentSymbols {
        #[arg(long)]
        db: PathBuf,
        #[arg(long)]
        workspace_root: PathBuf,
        #[arg(long)]
        package_path: PathBuf,
        #[arg(long)]
        file: PathBuf,
    },
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::RustDocumentSymbols {
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
            let store = GraphStore::connect(db).await?;
            store.migrate().await?;

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
    }

    Ok(())
}
