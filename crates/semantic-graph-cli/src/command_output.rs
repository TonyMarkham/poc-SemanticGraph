use crate::install::{CodexInstallReport, CodexUninstallReport};

pub enum CommandOutput {
    InstallCodex(CodexInstallReport),
    UninstallCodex(CodexUninstallReport),
}

impl CommandOutput {
    pub fn lines(&self) -> Vec<String> {
        match self {
            Self::InstallCodex(report) => report.lines(),
            Self::UninstallCodex(report) => report.lines(),
        }
    }
}
