use crate::install::CodexInstallReport;

pub enum CommandOutput {
    InstallCodex(CodexInstallReport),
}

impl CommandOutput {
    pub fn lines(&self) -> Vec<String> {
        match self {
            Self::InstallCodex(report) => report.lines(),
        }
    }
}
