use crate::{args::McpInstallMode, install::FileAction};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodexInstallReport {
    project_root: PathBuf,
    dry_run: bool,
    mcp_mode: McpInstallMode,
    actions: Vec<FileAction>,
}

impl CodexInstallReport {
    pub fn new(
        project_root: PathBuf,
        dry_run: bool,
        mcp_mode: McpInstallMode,
        actions: Vec<FileAction>,
    ) -> Self {
        Self {
            project_root,
            dry_run,
            mcp_mode,
            actions,
        }
    }

    pub fn actions(&self) -> &[FileAction] {
        &self.actions
    }

    pub fn has_refusals(&self) -> bool {
        self.actions
            .iter()
            .any(|action| action.kind() == crate::install::FileActionKind::Refuse)
    }

    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    pub fn lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        if self.dry_run {
            lines.push(format!(
                "semantic-graph install codex dry-run for {} (mcp: {}; no files written)",
                self.project_root.display(),
                self.mcp_mode
            ));
        } else {
            lines.push(format!(
                "semantic-graph install codex for {} (mcp: {})",
                self.project_root.display(),
                self.mcp_mode
            ));
        }
        for action in &self.actions {
            lines.push(action.line());
        }
        lines
    }
}
