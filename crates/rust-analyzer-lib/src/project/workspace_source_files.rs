use crate::model::RustWorkspaceModel;

use std::path::PathBuf;

pub fn workspace_source_files(model: &RustWorkspaceModel) -> Vec<PathBuf> {
    model
        .source_files
        .iter()
        .map(|source_file| source_file.path.clone())
        .collect()
}
