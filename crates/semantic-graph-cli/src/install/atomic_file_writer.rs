use crate::{SemanticGraphCliError, SemanticGraphCliResult, install::ProjectRoot};
use std::{
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};

static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

pub struct AtomicFileWriter;

impl AtomicFileWriter {
    pub fn write(
        project_root: &ProjectRoot,
        relative_path: &Path,
        content: &str,
    ) -> SemanticGraphCliResult<()> {
        project_root.ensure_parent_dir(relative_path)?;
        let target = project_root.target_path(relative_path)?;
        let parent = target.parent().ok_or_else(|| {
            SemanticGraphCliError::invalid_install_path(
                relative_path.to_path_buf(),
                "target path has no parent directory",
            )
        })?;
        let file_name = target.file_name().ok_or_else(|| {
            SemanticGraphCliError::invalid_install_path(
                relative_path.to_path_buf(),
                "target path has no file name",
            )
        })?;
        let temp_name = format!(
            ".{}.semantic-graph.tmp.{}.{}",
            file_name.to_string_lossy(),
            std::process::id(),
            TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed)
        );
        let temp_path = parent.join(temp_name);
        std::fs::write(&temp_path, content).map_err(|source| {
            SemanticGraphCliError::io("write temporary file", Some(temp_path.clone()), source)
        })?;
        std::fs::rename(&temp_path, &target).map_err(|source| {
            let _ = std::fs::remove_file(&temp_path);
            SemanticGraphCliError::io("rename temporary file", Some(target), source)
        })
    }
}
