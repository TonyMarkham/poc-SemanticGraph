use crate::files::{AssetWriteReport, DriftCheckReport};

pub enum CommandOutput {
    Generate(AssetWriteReport),
    Check(DriftCheckReport),
}

impl CommandOutput {
    pub fn lines(&self) -> Vec<String> {
        match self {
            Self::Generate(report) => report.lines(),
            Self::Check(report) => report.lines(),
        }
    }
}
