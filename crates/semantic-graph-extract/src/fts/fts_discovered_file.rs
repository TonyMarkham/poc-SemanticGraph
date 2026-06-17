use crate::fts::FtsFileLanguage;

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FtsDiscoveredFile {
    absolute_path: PathBuf,
    relative_path: String,
    language: FtsFileLanguage,
}

impl FtsDiscoveredFile {
    pub fn new(absolute_path: PathBuf, relative_path: String, language: FtsFileLanguage) -> Self {
        Self {
            absolute_path,
            relative_path,
            language,
        }
    }

    pub fn absolute_path(&self) -> &PathBuf {
        &self.absolute_path
    }

    pub fn relative_path(&self) -> &str {
        &self.relative_path
    }

    pub fn language(&self) -> FtsFileLanguage {
        self.language
    }
}
