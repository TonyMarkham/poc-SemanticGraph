use crate::args::McpInstallMode;
use clap::Args;
use std::path::PathBuf;

#[derive(Clone, Debug, Args)]
pub struct CodexInstallArgs {
    #[arg(long = "project", value_name = "DIR", default_value = ".")]
    project: PathBuf,

    #[arg(long = "database-path", value_name = "PATH")]
    database_path: Option<String>,

    #[arg(
        long = "mcp",
        value_enum,
        default_value_t = McpInstallMode::ReadOnly,
        help = "MCP config mode. 'disabled' writes a disabled project-local semantic_graph table."
    )]
    mcp: McpInstallMode,

    #[arg(long = "dry-run")]
    dry_run: bool,

    #[arg(long = "force")]
    force: bool,
}

impl CodexInstallArgs {
    pub fn project(&self) -> &PathBuf {
        &self.project
    }

    pub fn database_path(&self) -> Option<&str> {
        self.database_path.as_deref()
    }

    pub fn mcp(&self) -> McpInstallMode {
        self.mcp
    }

    pub fn dry_run(&self) -> bool {
        self.dry_run
    }

    pub fn force(&self) -> bool {
        self.force
    }
}
