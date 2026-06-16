use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CSharpProjectModel {
    pub project_path: PathBuf,
    pub source_files: Vec<PathBuf>,
}
