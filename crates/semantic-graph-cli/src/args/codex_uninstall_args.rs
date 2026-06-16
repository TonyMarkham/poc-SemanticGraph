use clap::Args;
use std::path::PathBuf;

#[derive(Clone, Debug, Args)]
pub struct CodexUninstallArgs {
    #[arg(long = "project", value_name = "DIR", default_value = ".")]
    project: PathBuf,

    #[arg(long = "dry-run")]
    dry_run: bool,

    #[arg(long = "force")]
    force: bool,
}

impl CodexUninstallArgs {
    pub fn project(&self) -> &PathBuf {
        &self.project
    }

    pub fn dry_run(&self) -> bool {
        self.dry_run
    }

    pub fn force(&self) -> bool {
        self.force
    }
}
