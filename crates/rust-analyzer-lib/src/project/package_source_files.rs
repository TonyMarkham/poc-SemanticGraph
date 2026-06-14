use crate::model::RustWorkspaceModel;

use std::path::{Path, PathBuf};

pub fn package_source_files(
    model: &RustWorkspaceModel,
    package_path: impl AsRef<Path>,
) -> Vec<PathBuf> {
    let package_path = package_path
        .as_ref()
        .canonicalize()
        .unwrap_or_else(|_| package_path.as_ref().to_path_buf());

    model
        .source_files
        .iter()
        .filter(|source_file| source_file.path.starts_with(&package_path))
        .map(|source_file| source_file.path.clone())
        .collect()
}
