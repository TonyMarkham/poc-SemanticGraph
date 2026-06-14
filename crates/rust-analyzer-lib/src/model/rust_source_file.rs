use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustSourceFile {
    pub path: PathBuf,
}
