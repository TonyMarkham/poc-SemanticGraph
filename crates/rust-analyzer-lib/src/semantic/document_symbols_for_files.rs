use crate::{RustAnalyzerLibResult, semantic::document_symbols_for_file::LoadedAnalysis};

use lsp_types::DocumentSymbol;
use std::path::{Path, PathBuf};

pub fn document_symbols_for_files(
    workspace_root: impl AsRef<Path>,
    file_paths: &[PathBuf],
) -> RustAnalyzerLibResult<Vec<(PathBuf, Vec<DocumentSymbol>)>> {
    let loaded = LoadedAnalysis::load(workspace_root.as_ref())?;
    file_paths
        .iter()
        .map(|file_path| {
            loaded
                .document_symbols_for_path(file_path)
                .map(|symbols| (file_path.clone(), symbols))
        })
        .collect()
}
