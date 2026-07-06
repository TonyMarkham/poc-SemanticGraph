use crate::{
    RustAnalyzerLibError, RustAnalyzerLibResult,
    model::{
        ResolvedCallTarget, ResolvedOutgoingCallSet, ResolvedReferenceSet, ResolvedReferenceTarget,
    },
    semantic::{
        analysis_context::AnalysisContext, document_symbols_for_file::document_symbols_for_path,
        file_semantic_result::FileSemanticResult, file_semantic_work::FileSemanticWork,
        outgoing_calls_for_symbols::outgoing_calls_for_target, progress_callback::ProgressCallback,
        references_for_symbols::references_for_target,
    },
};

use ide::{AnalysisHost, FileId};
use load_cargo::{LoadCargoConfig, ProcMacroServerChoice};
use lsp_types::DocumentSymbol;
use paths::AbsPathBuf;
use project_model::{CargoConfig, ProjectManifest, ProjectWorkspace};
use std::path::{Path, PathBuf};
use vfs::{FileExcluded, VfsPath};

pub struct LoadedAnalysis {
    pub(super) analysis: ide::Analysis,
    pub(super) vfs: vfs::Vfs,
}

impl LoadedAnalysis {
    pub fn load(workspace_root: &Path) -> RustAnalyzerLibResult<Self> {
        let workspace_root = absolute_path(workspace_root, "canonicalize workspace root")?;
        let abs_root = AbsPathBuf::assert_utf8(workspace_root);
        let manifest = ProjectManifest::discover_single(&abs_root).map_err(|source| {
            RustAnalyzerLibError::project("discover Rust project manifest for analysis", source)
        })?;
        let cargo_config = CargoConfig {
            set_test: true,
            ..CargoConfig::default()
        };
        let project_workspace =
            ProjectWorkspace::load(manifest, &cargo_config, &|_| {}).map_err(|source| {
                RustAnalyzerLibError::project(
                    "load rust-analyzer project workspace for analysis",
                    source,
                )
            })?;
        let load_config = LoadCargoConfig {
            load_out_dirs_from_check: false,
            with_proc_macro_server: ProcMacroServerChoice::None,
            prefill_caches: false,
            num_worker_threads: 1,
            proc_macro_processes: 1,
        };
        let (db, vfs, _proc_macro) =
            load_cargo::load_workspace(project_workspace, &cargo_config.extra_env, &load_config)
                .map_err(|source| {
                    RustAnalyzerLibError::project(
                        "load rust-analyzer workspace database for analysis",
                        source,
                    )
                })?;
        let host = AnalysisHost::with_database(db);

        Ok(Self {
            analysis: host.analysis(),
            vfs,
        })
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

    pub(super) fn file_id_for_path(&self, file_path: &Path) -> RustAnalyzerLibResult<FileId> {
        let abs_path = AbsPathBuf::assert_utf8(file_path.to_path_buf());
        let vfs_path = VfsPath::from(abs_path);
        match self.vfs.file_id(&vfs_path) {
            Some((file_id, FileExcluded::No)) => Ok(file_id),
            Some((_file_id, FileExcluded::Yes)) => Err(RustAnalyzerLibError::invalid_path(
                file_path,
                "source file is excluded from the rust-analyzer VFS",
            )),
            None => Err(RustAnalyzerLibError::invalid_path(
                file_path,
                "source file was not loaded into the rust-analyzer VFS",
            )),
        }
    }

    pub(super) fn file_path_for_id(&self, file_id: FileId) -> Option<PathBuf> {
        self.vfs
            .file_path(file_id)
            .as_path()
            .map(|abs_path| PathBuf::from(abs_path.to_path_buf()))
    }
}

impl AnalysisContext for LoadedAnalysis {
    fn analysis(&self) -> &ide::Analysis {
        &self.analysis
    }

    fn file_id_for_path(&self, file_path: &Path) -> RustAnalyzerLibResult<FileId> {
        LoadedAnalysis::file_id_for_path(self, file_path)
    }

    fn file_path_for_id(&self, file_id: FileId) -> Option<PathBuf> {
        LoadedAnalysis::file_path_for_id(self, file_id)
    }
}

pub(super) fn absolute_path(path: &Path, context: &'static str) -> RustAnalyzerLibResult<PathBuf> {
    path.canonicalize()
        .map_err(|source| RustAnalyzerLibError::io(context, Some(path.to_path_buf()), source))
}
