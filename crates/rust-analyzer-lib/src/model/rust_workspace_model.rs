use crate::model::{RustPackage, RustSourceFile};

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustWorkspaceModel {
    pub workspace_root: PathBuf,
    pub packages: Vec<RustPackage>,
    pub source_files: Vec<RustSourceFile>,
}
