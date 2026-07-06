use crate::fts::FtsExtractionOptions;

use semantic_graph_config::FtsConfig;
use semantic_graph_db_manager::WriteHandle;
use std::path::Path;

pub struct FtsExtractionRunRequest<'a> {
    pub(crate) store: &'a WriteHandle,
    pub(crate) workspace_root: &'a Path,
    pub(crate) db_path: &'a Path,
    pub(crate) index_path: &'a Path,
    pub(crate) config: &'a FtsConfig,
    pub(crate) options: FtsExtractionOptions,
    pub(crate) analysis_workers: usize,
}

impl<'a> FtsExtractionRunRequest<'a> {
    pub fn new(
        store: &'a WriteHandle,
        workspace_root: &'a Path,
        db_path: &'a Path,
        index_path: &'a Path,
        config: &'a FtsConfig,
        options: FtsExtractionOptions,
        analysis_workers: usize,
    ) -> Self {
        Self {
            store,
            workspace_root,
            db_path,
            index_path,
            config,
            options,
            analysis_workers,
        }
    }
}
