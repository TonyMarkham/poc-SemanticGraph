use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManagedFile {
    relative_path: PathBuf,
    content: String,
}

impl ManagedFile {
    pub fn new(relative_path: PathBuf, content: String) -> Self {
        Self {
            relative_path,
            content,
        }
    }

    pub fn relative_path(&self) -> &Path {
        &self.relative_path
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn bytes(&self) -> &[u8] {
        self.content.as_bytes()
    }
}
