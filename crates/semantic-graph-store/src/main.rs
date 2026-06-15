use semantic_graph_config::{LoadOptions, discover_config, load_config, resolve_database_path};
use semantic_graph_db_manager::{Config, WriteManager};
use semantic_graph_store::{GraphStore, GraphStoreResult};

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(about = "Semantic graph storage prototype")]
struct Cli {
    #[arg(long, global = true)]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Init {
        #[arg(long)]
        db: Option<PathBuf>,
    },
    DemoSeed {
        #[arg(long)]
        db: Option<PathBuf>,
        #[arg(long)]
        root_uri: String,
    },
    Stats {
        #[arg(long)]
        db: Option<PathBuf>,
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
    let config = cli.config;

    match cli.command {
        Command::Init { db } => {
            let db = resolve_cli_database_path(db, &config)?;
            let writer_config = resolve_cli_writer_config(&config)?;
            let writer = WriteManager::start_with_config(db, writer_config)
                .await
                .map_err(semantic_graph_store::GraphStoreError::db_manager)?;
            writer
                .migrate()
                .await
                .map_err(semantic_graph_store::GraphStoreError::db_manager)?;
            writer
                .shutdown()
                .await
                .map_err(semantic_graph_store::GraphStoreError::db_manager)?;
            println!("initialized");
        }
        Command::DemoSeed { db, root_uri } => {
            let db = resolve_cli_database_path(db, &config)?;
            let writer_config = resolve_cli_writer_config(&config)?;
            let writer = WriteManager::start_with_config(db, writer_config)
                .await
                .map_err(semantic_graph_store::GraphStoreError::db_manager)?;
            writer
                .migrate()
                .await
                .map_err(semantic_graph_store::GraphStoreError::db_manager)?;
            let summary = writer
                .demo_seed(&root_uri)
                .await
                .map_err(semantic_graph_store::GraphStoreError::db_manager)?;
            writer
                .shutdown()
                .await
                .map_err(semantic_graph_store::GraphStoreError::db_manager)?;
            println!(
                "seeded workspace={} run={} file={} edge={}",
                summary.workspace_id, summary.run_id, summary.file_id, summary.edge_id
            );
        }
        Command::Stats { db } => {
            let db = resolve_cli_database_path(db, &config)?;
            let writer_config = resolve_cli_writer_config(&config)?;
            let writer = WriteManager::start_with_config(&db, writer_config)
                .await
                .map_err(semantic_graph_store::GraphStoreError::db_manager)?;
            writer
                .migrate()
                .await
                .map_err(semantic_graph_store::GraphStoreError::db_manager)?;
            writer
                .shutdown()
                .await
                .map_err(semantic_graph_store::GraphStoreError::db_manager)?;
            let store = GraphStore::connect(db).await?;
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

fn resolve_cli_writer_config(config: &Option<PathBuf>) -> GraphStoreResult<Config> {
    let config_path = match config {
        Some(path) => Some(path.clone()),
        None => discover_config(PathBuf::from("."))
            .map_err(semantic_graph_store::GraphStoreError::config)?,
    };

    let Some(config_path) = config_path else {
        return Ok(Config::default());
    };

    let config = load_config(config_path).map_err(semantic_graph_store::GraphStoreError::config)?;
    Ok(Config::from(config.writer()))
}

fn resolve_cli_database_path(
    db: Option<PathBuf>,
    config: &Option<PathBuf>,
) -> GraphStoreResult<PathBuf> {
    resolve_database_path(LoadOptions {
        explicit_database_path: db,
        explicit_config_path: config.clone(),
        discovery_start_dir: None,
        default_database_path: None,
    })
    .map(|resolved| resolved.into_path())
    .map_err(semantic_graph_store::GraphStoreError::config)
}

#[cfg(test)]
mod cli_tests {
    use crate::{Cli, Command, resolve_cli_database_path};
    use clap::Parser;
    use std::{
        error::Error,
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn stats_accepts_config_without_db() -> Result<(), Box<dyn Error>> {
        let cli = Cli::try_parse_from([
            "semantic-graph-store",
            "stats",
            "--config",
            ".refactor-radar/config.toml",
        ])?;

        assert_eq!(
            cli.config,
            Some(PathBuf::from(".refactor-radar/config.toml"))
        );
        match cli.command {
            Command::Stats { db } => assert_eq!(db, None),
            _ => return Err("expected stats command".into()),
        }

        Ok(())
    }

    #[test]
    fn stats_uses_config_path() -> Result<(), Box<dyn Error>> {
        let root = temp_dir("store-stats-config")?;
        let config_path = write_config(&root, "path = \".local/store.db\"")?;

        let resolved = resolve_cli_database_path(None, &Some(config_path.clone()))?;

        assert_eq!(
            resolved,
            config_path
                .parent()
                .ok_or("expected config parent")?
                .join(".local/store.db")
        );
        Ok(())
    }

    fn write_config(root: &Path, database_line: &str) -> Result<PathBuf, Box<dyn Error>> {
        let config_dir = root.join(".refactor-radar");
        fs::create_dir_all(&config_dir)?;
        let config_path = config_dir.join("config.toml");
        fs::write(&config_path, format!("[database]\n{database_line}\n"))?;
        Ok(config_path)
    }

    fn temp_dir(name: &str) -> Result<PathBuf, Box<dyn Error>> {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let path = std::env::temp_dir().join(format!(
            "semantic-graph-store-{name}-{}-{stamp}",
            std::process::id()
        ));
        fs::create_dir_all(&path)?;
        Ok(path)
    }
}
