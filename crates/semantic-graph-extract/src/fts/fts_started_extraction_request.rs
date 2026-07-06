use crate::fts::{FtsExtractionOptions, FtsStartedRun};

use semantic_graph_config::FtsConfig;
use semantic_graph_db_manager::WriteHandle;
use std::path::Path;

pub(crate) struct FtsStartedExtractionRequest<'a> {
    pub(crate) store: &'a WriteHandle,
    pub(crate) workspace_root: &'a Path,
    pub(crate) db_path: &'a Path,
    pub(crate) index_path: &'a Path,
    pub(crate) started_run: FtsStartedRun,
    pub(crate) config: &'a FtsConfig,
    pub(crate) options: FtsExtractionOptions,
}
