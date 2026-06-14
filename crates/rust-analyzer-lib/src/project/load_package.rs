use crate::{RustAnalyzerLibResult, model::RustWorkspaceModel};

use std::path::Path;

pub fn load_package(
    workspace_root: impl AsRef<Path>,
    _package_path: impl AsRef<Path>,
) -> RustAnalyzerLibResult<RustWorkspaceModel> {
    crate::project::load_workspace(workspace_root)
}
