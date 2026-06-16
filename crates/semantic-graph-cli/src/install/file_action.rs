use crate::install::FileActionKind;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileAction {
    kind: FileActionKind,
    relative_path: PathBuf,
}

impl FileAction {
    pub fn new(kind: FileActionKind, relative_path: PathBuf) -> Self {
        Self {
            kind,
            relative_path,
        }
    }

    pub fn kind(&self) -> FileActionKind {
        self.kind
    }

    pub fn relative_path(&self) -> &Path {
        &self.relative_path
    }

    pub fn line(&self) -> String {
        format!("{}: {}", self.kind.label(), self.relative_path.display())
    }
}
