use clap::Subcommand;
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub enum Command {
    #[command(name = "rust-file")]
    RustFile {
        #[arg(long)]
        db: Option<PathBuf>,
        #[arg(long, value_name = "WORKSPACE_ROOT", default_value = ".")]
        workspace_root: PathBuf,
        #[arg(long)]
        calls: bool,
        #[arg(long)]
        references: bool,
        #[arg(long)]
        symbols: bool,
        #[arg(value_name = "FILE")]
        file: PathBuf,
    },

    #[command(name = "rust-file-deleted")]
    RustFileDeleted {
        #[arg(long)]
        db: Option<PathBuf>,
        #[arg(long, value_name = "WORKSPACE_ROOT", default_value = ".")]
        workspace_root: PathBuf,
        #[arg(value_name = "FILE")]
        file: PathBuf,
    },

    #[command(name = "rust-crate")]
    RustCrate {
        #[arg(long)]
        db: Option<PathBuf>,
        #[arg(long, value_name = "WORKSPACE_ROOT", default_value = ".")]
        workspace_root: PathBuf,
        #[arg(long)]
        analysis_workers: Option<usize>,
        #[arg(long)]
        calls: bool,
        #[arg(long)]
        references: bool,
        #[arg(long)]
        symbols: bool,
        #[arg(value_name = "PATH")]
        package_path: PathBuf,
    },

    #[command(name = "rust-workspace")]
    RustWorkspace {
        #[arg(long)]
        db: Option<PathBuf>,
        #[arg(long, value_name = "WORKSPACE_ROOT", default_value = ".")]
        workspace_root: PathBuf,
        #[arg(long)]
        analysis_workers: Option<usize>,
        #[arg(long)]
        calls: bool,
        #[arg(long)]
        references: bool,
        #[arg(long)]
        symbols: bool,
    },
}
