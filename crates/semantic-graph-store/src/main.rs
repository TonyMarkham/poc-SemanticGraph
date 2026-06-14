use semantic_graph_store::{GraphStore, GraphStoreResult};

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(about = "Semantic graph storage prototype")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Init {
        #[arg(long)]
        db: PathBuf,
    },
    DemoSeed {
        #[arg(long)]
        db: PathBuf,
        #[arg(long)]
        root_uri: String,
    },
    Stats {
        #[arg(long)]
        db: PathBuf,
    },
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

async fn run() -> GraphStoreResult<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Init { db } => {
            let store = GraphStore::connect(db).await?;
            store.migrate().await?;
            println!("initialized");
        }
        Command::DemoSeed { db, root_uri } => {
            let store = GraphStore::connect(db).await?;
            store.migrate().await?;
            let summary = store.demo_seed(&root_uri).await?;
            println!(
                "seeded workspace={} run={} file={} edge={}",
                summary.workspace_id, summary.run_id, summary.file_id, summary.edge_id
            );
        }
        Command::Stats { db } => {
            let store = GraphStore::connect(db).await?;
            store.migrate().await?;
            let stats = store.stats().await?;
            println!("workspaces={}", stats.workspaces);
            println!("extraction_runs={}", stats.extraction_runs);
            println!("files={}", stats.files);
            println!("nodes={}", stats.nodes);
            println!("edges={}", stats.edges);
            println!("occurrences={}", stats.occurrences);
            println!("edge_evidence={}", stats.edge_evidence);
        }
    }

    Ok(())
}
