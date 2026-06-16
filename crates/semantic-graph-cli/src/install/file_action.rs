use crate::install::FileActionKind;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileAction {
    kind: FileActionKind,
    relative_path: PathBuf,
    reason: Option<String>,
}

impl FileAction {
    pub fn new(kind: FileActionKind, relative_path: PathBuf) -> Self {
        Self {
            kind,
            relative_path,
            reason: None,
        }
    }

    pub fn refused(relative_path: PathBuf, reason: impl Into<String>) -> Self {
        Self {
            kind: FileActionKind::Refuse,
            relative_path,
            reason: Some(reason.into()),
        }
    }

    pub fn kind(&self) -> FileActionKind {
        self.kind
    }

    pub fn relative_path(&self) -> &Path {
        &self.relative_path
    }

    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }

    pub fn line(&self) -> String {
        match &self.reason {
            Some(reason) => format!(
                "{}: {} ({reason})",
                self.kind.label(),
                self.relative_path.display()
            ),
            None => format!("{}: {}", self.kind.label(), self.relative_path.display()),
        }
    }
}
