use clap::Subcommand;
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub enum Command {
    #[command(name = "fts")]
    Fts {
        #[arg(long)]
        db: Option<PathBuf>,
        #[arg(long)]
        analysis_workers: Option<usize>,
        #[arg(long)]
        no_rust: bool,
        #[arg(long)]
        no_csharp: bool,
        #[arg(long)]
        no_submodules: bool,
    },

    #[command(name = "fts-tantivy")]
    FtsTantivy {
        #[arg(long)]
        db: PathBuf,
        #[arg(long)]
        analysis_workers: Option<usize>,
        #[arg(long)]
        no_rust: bool,
        #[arg(long)]
        no_csharp: bool,
        #[arg(long)]
        no_submodules: bool,
    },

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

    #[command(name = "csharp-file")]
    CSharpFile {
        #[arg(long)]
        db: Option<PathBuf>,
        #[arg(long, value_name = "SLN_OR_SLNX")]
        solution: Option<PathBuf>,
        #[arg(long = "csharp-ls", value_name = "BINARY")]
        csharp_ls: Option<PathBuf>,
        #[arg(long)]
        calls: bool,
        #[arg(long)]
        references: bool,
        #[arg(long)]
        symbols: bool,
        #[arg(value_name = "FILE")]
        file: PathBuf,
    },

    #[command(name = "csharp-file-deleted")]
    CSharpFileDeleted {
        #[arg(long)]
        db: Option<PathBuf>,
        #[arg(long, value_name = "SLN_OR_SLNX")]
        solution: Option<PathBuf>,
        #[arg(long = "csharp-ls", value_name = "BINARY")]
        csharp_ls: Option<PathBuf>,
        #[arg(value_name = "FILE")]
        file: PathBuf,
    },

    #[command(name = "csharp-project")]
    CSharpProject {
        #[arg(long)]
        db: Option<PathBuf>,
        #[arg(long, value_name = "SLN_OR_SLNX")]
        solution: Option<PathBuf>,
        #[arg(long = "csharp-ls", value_name = "BINARY")]
        csharp_ls: Option<PathBuf>,
        #[arg(long)]
        process_workers: Option<usize>,
        #[arg(long)]
        calls: bool,
        #[arg(long)]
        references: bool,
        #[arg(long)]
        symbols: bool,
        #[arg(value_name = "PROJECT_OR_ROOT")]
        project_or_root: PathBuf,
    },

    #[command(name = "csharp-solution")]
    CSharpSolution {
        #[arg(long)]
        db: Option<PathBuf>,
        #[arg(long, value_name = "SLN_OR_SLNX")]
        solution: Option<PathBuf>,
        #[arg(long = "csharp-ls", value_name = "BINARY")]
        csharp_ls: Option<PathBuf>,
        #[arg(long)]
        process_workers: Option<usize>,
        #[arg(long)]
        calls: bool,
        #[arg(long)]
        references: bool,
        #[arg(long)]
        symbols: bool,
    },
}
