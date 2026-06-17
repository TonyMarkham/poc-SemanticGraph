use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkspaceFileHash {
    pub(crate) file_path: PathBuf,
    pub(crate) uri: String,
    pub(crate) content_hash: String,
}
