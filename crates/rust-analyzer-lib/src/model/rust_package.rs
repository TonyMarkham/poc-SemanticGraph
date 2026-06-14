use crate::model::RustTarget;

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustPackage {
    pub name: String,
    pub manifest_path: PathBuf,
    pub package_root: PathBuf,
    pub is_workspace_member: bool,
    pub targets: Vec<RustTarget>,
}
