use crate::{SoulLspLibResult, semantic::loaded_soul_workspace::LoadedSoulWorkspace};

use lsp_types::DocumentSymbol;
use std::path::Path;

pub fn document_symbols_for_file(
    workspace_root: impl AsRef<Path>,
    file_path: impl AsRef<Path>,
) -> SoulLspLibResult<Vec<DocumentSymbol>> {
    let loaded = LoadedSoulWorkspace::load(workspace_root)?;
    loaded.document_symbols_for_file(file_path.as_ref())
}
