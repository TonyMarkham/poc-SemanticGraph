use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct DriftCheckReport {
    pub checked: Vec<PathBuf>,
}

impl DriftCheckReport {
    pub fn lines(&self) -> Vec<String> {
        vec![format!(
            "Codex asset drift check passed: checked={} files",
            self.checked.len()
        )]
    }
}
