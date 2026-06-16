use crate::install::{FileAction, FileActionKind};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodexUninstallReport {
    project_root: PathBuf,
    dry_run: bool,
    actions: Vec<FileAction>,
}

impl CodexUninstallReport {
    pub fn new(project_root: PathBuf, dry_run: bool, actions: Vec<FileAction>) -> Self {
        Self {
            project_root,
            dry_run,
            actions,
        }
    }

    pub fn actions(&self) -> &[FileAction] {
        &self.actions
    }

    pub fn has_refusals(&self) -> bool {
        self.actions
            .iter()
            .any(|action| action.kind() == FileActionKind::Refuse)
    }

    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    pub fn lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        if self.dry_run {
            lines.push(format!(
                "semantic-graph uninstall codex dry-run for {} (no files written or deleted)",
                self.project_root.display()
            ));
        } else {
            lines.push(format!(
                "semantic-graph uninstall codex for {}",
                self.project_root.display()
            ));
        }
        for action in &self.actions {
            lines.push(action.line());
        }
        lines
    }
}
