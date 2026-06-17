use crate::RustAnalyzerLibResult;

use ide::FileId;
use std::path::{Path, PathBuf};

pub(super) trait AnalysisContext {
    fn analysis(&self) -> &ide::Analysis;

    fn file_id_for_path(&self, file_path: &Path) -> RustAnalyzerLibResult<FileId>;

    fn file_path_for_id(&self, file_id: FileId) -> Option<PathBuf>;
}
