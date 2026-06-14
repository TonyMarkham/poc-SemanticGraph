use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceBatchRequest {
    pub workspace_root: PathBuf,
    pub package_path: PathBuf,
    pub file_paths: Vec<PathBuf>,
}
