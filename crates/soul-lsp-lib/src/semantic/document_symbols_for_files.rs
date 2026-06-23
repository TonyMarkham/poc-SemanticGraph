use crate::{SoulLspLibResult, semantic::DocumentSymbolItems};

use crate::semantic::loaded_soul_workspace::LoadedSoulWorkspace;
use std::path::{Path, PathBuf};

pub fn document_symbols_for_files(
    workspace_root: impl AsRef<Path>,
    file_paths: &[PathBuf],
) -> SoulLspLibResult<DocumentSymbolItems> {
    let loaded = LoadedSoulWorkspace::load(workspace_root)?;
    loaded.document_symbols_for_files(file_paths.to_vec())
}
