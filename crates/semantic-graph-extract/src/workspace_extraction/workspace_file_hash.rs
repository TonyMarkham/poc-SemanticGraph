use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceFileHash {
    pub file_path: PathBuf,
    pub uri: String,
    pub content_hash: String,
}
