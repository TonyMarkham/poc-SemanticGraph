use crate::{
    RustAnalyzerLibResult,
    model::{
        ResolvedCallTarget, ResolvedOutgoingCallSet, ResolvedReferenceSet, ResolvedReferenceTarget,
    },
    semantic::{
        analysis_context::AnalysisContext, analysis_path_index::AnalysisPathIndex,
        document_symbols_for_file::document_symbols_for_path,
        file_semantic_result::FileSemanticResult, file_semantic_work::FileSemanticWork,
        outgoing_calls_for_symbols::outgoing_calls_for_target, progress_callback::ProgressCallback,
        references_for_symbols::references_for_target,
    },
};

use ide::FileId;
use lsp_types::DocumentSymbol;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

pub struct SharedAnalysisSnapshot {
    analysis: ide::Analysis,
    path_index: Arc<AnalysisPathIndex>,
}

impl SharedAnalysisSnapshot {
    pub(super) fn new(analysis: ide::Analysis, path_index: Arc<AnalysisPathIndex>) -> Self {
        Self {
            analysis,
            path_index,
        }
    }

    pub fn document_symbols_for_files(
        &self,
        file_paths: &[PathBuf],
    ) -> RustAnalyzerLibResult<Vec<(PathBuf, Vec<DocumentSymbol>)>> {
        self.document_symbols_for_files_internal(file_paths, None)
    }

    pub fn document_symbols_for_files_with_progress(
        &self,
        file_paths: &[PathBuf],
        progress: ProgressCallback,
    ) -> RustAnalyzerLibResult<Vec<(PathBuf, Vec<DocumentSymbol>)>> {
        self.document_symbols_for_files_internal(file_paths, Some(progress))
    }

    fn document_symbols_for_files_internal(
        &self,
        file_paths: &[PathBuf],
        progress: Option<ProgressCallback>,
    ) -> RustAnalyzerLibResult<Vec<(PathBuf, Vec<DocumentSymbol>)>> {
        file_paths
            .iter()
            .map(|file_path| {
                document_symbols_for_path(self, file_path).map(|symbols| {
                    if let Some(progress) = &progress {
                        progress();
                    }
                    (file_path.clone(), symbols)
                })
            })
            .collect()
    }

    pub fn references_for_symbol(
        &self,
        target: &ResolvedReferenceTarget,
    ) -> RustAnalyzerLibResult<ResolvedReferenceSet> {
        references_for_target(self, target)
    }

    pub fn outgoing_calls_for_symbol(
        &self,
        caller: &ResolvedCallTarget,
    ) -> RustAnalyzerLibResult<ResolvedOutgoingCallSet> {
        outgoing_calls_for_target(self, caller)
    }

    pub fn file_semantic_work(
        &self,
        work: FileSemanticWork,
    ) -> RustAnalyzerLibResult<FileSemanticResult> {
        let mut reference_sets = Vec::with_capacity(work.reference_targets.len());
        for target in &work.reference_targets {
            reference_sets.push(references_for_target(self, target)?);
        }

        let mut call_sets = Vec::with_capacity(work.call_targets.len());
        for target in &work.call_targets {
            call_sets.push(outgoing_calls_for_target(self, target)?);
        }

        Ok(FileSemanticResult {
            file_path: work.file_path,
            reference_sets,
            call_sets,
        })
    }
}

impl AnalysisContext for SharedAnalysisSnapshot {
    fn analysis(&self) -> &ide::Analysis {
        &self.analysis
    }

    fn file_id_for_path(&self, file_path: &Path) -> RustAnalyzerLibResult<FileId> {
        self.path_index.file_id_for_path(file_path)
    }

    fn file_path_for_id(&self, file_id: FileId) -> Option<PathBuf> {
        self.path_index.file_path_for_id(file_id)
    }
}
