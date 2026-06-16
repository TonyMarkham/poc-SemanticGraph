use crate::error::{AgentAssetsError, AgentAssetsResult};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub(crate) fn collect_files(root: &Path) -> AgentAssetsResult<Vec<PathBuf>> {
    let mut files = Vec::new();
    if !root.exists() {
        return Ok(files);
    }

    collect_files_from(root, root, &mut files)?;
    files.sort_by_key(|path| path.to_string_lossy().to_string());
    Ok(files)
}

fn collect_files_from(
    root: &Path,
    current: &Path,
    files: &mut Vec<PathBuf>,
) -> AgentAssetsResult<()> {
    let entries = fs::read_dir(current).map_err(|source| {
        AgentAssetsError::io("read directory", Some(current.to_path_buf()), source)
    })?;

    for entry in entries {
        let entry = entry.map_err(|source| {
            AgentAssetsError::io("read directory entry", Some(current.to_path_buf()), source)
        })?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|source| {
            AgentAssetsError::io("read directory entry type", Some(path.clone()), source)
        })?;
        if file_type.is_dir() {
            collect_files_from(root, &path, files)?;
        } else {
            let relative = path.strip_prefix(root).map_err(|_| {
                AgentAssetsError::invalid_manifest(format!(
                    "expected artifact path {} is outside {}",
                    path.display(),
                    root.display()
                ))
            })?;
            files.push(relative.to_path_buf());
        }
    }

    Ok(())
}
