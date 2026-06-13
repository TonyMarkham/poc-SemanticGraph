use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentSymbolRequest {
    pub workspace_root: PathBuf,
    pub package_path: PathBuf,
    pub file_path: PathBuf,
}
