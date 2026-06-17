use crate::{RustAnalyzerLibError, RustAnalyzerLibResult};

use ide::FileId;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

pub struct AnalysisPathIndex {
    file_ids_by_path: HashMap<PathBuf, FileId>,
    file_paths_by_id: HashMap<FileId, PathBuf>,
}

impl AnalysisPathIndex {
    pub(super) fn from_vfs(vfs: &vfs::Vfs) -> Self {
        let mut file_ids_by_path = HashMap::new();
        let mut file_paths_by_id = HashMap::new();

        for (file_id, vfs_path) in vfs.iter() {
            let Some(abs_path) = vfs_path.as_path() else {
                continue;
            };
            let file_path = PathBuf::from(abs_path.to_path_buf());
            file_ids_by_path.insert(file_path.clone(), file_id);
            file_paths_by_id.insert(file_id, file_path);
        }

        Self {
            file_ids_by_path,
            file_paths_by_id,
        }
    }

    pub(super) fn file_id_for_path(&self, file_path: &Path) -> RustAnalyzerLibResult<FileId> {
        self.file_ids_by_path
            .get(file_path)
            .copied()
            .ok_or_else(|| {
                RustAnalyzerLibError::invalid_path(
                    file_path,
                    "source file was not loaded into the shared rust-analyzer path index",
                )
            })
    }

    pub(super) fn file_path_for_id(&self, file_id: FileId) -> Option<PathBuf> {
        self.file_paths_by_id.get(&file_id).cloned()
    }
}
