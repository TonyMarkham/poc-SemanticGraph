use crate::{
    RustAnalyzerLibResult,
    semantic::{loaded_analysis::LoadedAnalysis, progress_callback::ProgressCallback},
};

use lsp_types::DocumentSymbol;
use std::path::{Path, PathBuf};

pub fn document_symbols_for_files(
    workspace_root: impl AsRef<Path>,
    file_paths: &[PathBuf],
) -> RustAnalyzerLibResult<Vec<(PathBuf, Vec<DocumentSymbol>)>> {
    let loaded = LoadedAnalysis::load(workspace_root.as_ref())?;
    loaded.document_symbols_for_files(file_paths)
}

pub fn document_symbols_for_files_with_progress(
    workspace_root: impl AsRef<Path>,
    file_paths: &[PathBuf],
    progress: ProgressCallback,
) -> RustAnalyzerLibResult<Vec<(PathBuf, Vec<DocumentSymbol>)>> {
    let loaded = LoadedAnalysis::load(workspace_root.as_ref())?;
    loaded.document_symbols_for_files_with_progress(file_paths, progress)
}
