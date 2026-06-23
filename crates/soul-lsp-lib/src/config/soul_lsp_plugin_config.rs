use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoulLspPluginConfig {
    language: String,
    path: PathBuf,
}

impl SoulLspPluginConfig {
    pub fn new(language: String, path: PathBuf) -> Self {
        Self { language, path }
    }

    pub fn language(&self) -> &str {
        &self.language
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}
