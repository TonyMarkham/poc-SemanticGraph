use crate::{
    RustAnalyzerLibError, RustAnalyzerLibResult,
    semantic::{
        analysis_path_index::AnalysisPathIndex, loaded_analysis::absolute_path,
        shared_analysis_snapshot::SharedAnalysisSnapshot,
    },
};

use ide::AnalysisHost;
use load_cargo::{LoadCargoConfig, ProcMacroServerChoice};
use paths::AbsPathBuf;
use project_model::{CargoConfig, ProjectManifest, ProjectWorkspace};
use std::{path::Path, sync::Arc};

pub struct SharedAnalysisHost {
    host: AnalysisHost,
    path_index: Arc<AnalysisPathIndex>,
}

impl SharedAnalysisHost {
    pub fn load(workspace_root: &Path) -> RustAnalyzerLibResult<Self> {
        let workspace_root = absolute_path(workspace_root, "canonicalize workspace root")?;
        let abs_root = AbsPathBuf::assert_utf8(workspace_root);
        let manifest = ProjectManifest::discover_single(&abs_root).map_err(|source| {
            RustAnalyzerLibError::project(
                "discover Rust project manifest for shared analysis",
                source,
            )
        })?;
        let cargo_config = CargoConfig {
            set_test: true,
            ..CargoConfig::default()
        };
        let project_workspace =
            ProjectWorkspace::load(manifest, &cargo_config, &|_| {}).map_err(|source| {
                RustAnalyzerLibError::project(
                    "load rust-analyzer project workspace for shared analysis",
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
                        "load rust-analyzer workspace database for shared analysis",
                        source,
                    )
                })?;
        let path_index = Arc::new(AnalysisPathIndex::from_vfs(&vfs));
        let host = AnalysisHost::with_database(db);

        Ok(Self { host, path_index })
    }

    pub fn snapshot(&self) -> SharedAnalysisSnapshot {
        SharedAnalysisSnapshot::new(self.host.analysis(), Arc::clone(&self.path_index))
    }
}
