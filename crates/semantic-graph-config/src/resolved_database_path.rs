use crate::ResolvedDatabasePathSource;

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedDatabasePath {
    path: PathBuf,
    source: ResolvedDatabasePathSource,
}

impl ResolvedDatabasePath {
    pub fn new(path: PathBuf, source: ResolvedDatabasePathSource) -> Self {
        Self { path, source }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn source(&self) -> ResolvedDatabasePathSource {
        self.source
    }

    pub fn into_path(self) -> PathBuf {
        self.path
    }
}
