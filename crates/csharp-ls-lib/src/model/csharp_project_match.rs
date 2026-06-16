use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CSharpProjectMatch {
    pub project_path: PathBuf,
    pub file_path: PathBuf,
}
