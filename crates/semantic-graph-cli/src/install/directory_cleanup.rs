use crate::install::{FileAction, FileActionKind, ProjectRoot};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

pub struct DirectoryCleanup;

impl DirectoryCleanup {
    pub fn plan(project_root: &ProjectRoot, actions: &[FileAction]) -> Vec<FileAction> {
        let deleted_files = actions
            .iter()
            .filter(|action| action.kind().deletes_file())
            .map(|action| action.relative_path().to_path_buf())
            .collect::<BTreeSet<_>>();
        let mut candidate_dirs = deleted_files
            .iter()
            .filter_map(|path| path.parent())
            .filter(|path| !path.as_os_str().is_empty())
            .map(Path::to_path_buf)
            .collect::<BTreeSet<_>>();
        let mut removed_dirs = BTreeSet::new();

        loop {
            let mut changed = false;
            let candidates = candidate_dirs.iter().cloned().collect::<Vec<_>>();
            for candidate in candidates {
                if removed_dirs.contains(&candidate) {
                    continue;
                }
                if Self::directory_will_be_empty(
                    project_root,
                    &candidate,
                    &deleted_files,
                    &removed_dirs,
                ) {
                    if let Some(parent) = candidate.parent()
                        && !parent.as_os_str().is_empty()
                    {
                        candidate_dirs.insert(parent.to_path_buf());
                    }
                    removed_dirs.insert(candidate);
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }

        let mut removed_dirs = removed_dirs.into_iter().collect::<Vec<_>>();
        removed_dirs.sort_by(|left, right| {
            right
                .components()
                .count()
                .cmp(&left.components().count())
                .then_with(|| left.cmp(right))
        });

        removed_dirs
            .into_iter()
            .map(|path| FileAction::new(FileActionKind::RemoveDir, path))
            .collect()
    }

    pub fn cleanup_after_file_delete(project_root: &ProjectRoot, relative_path: &Path) {
        let mut current = match relative_path.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
            _ => return,
        };

        loop {
            if !Self::try_remove_empty_dir(project_root, &current) {
                return;
            }
            current = match current.parent() {
                Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
                _ => return,
            };
        }
    }

    fn try_remove_empty_dir(project_root: &ProjectRoot, relative_path: &Path) -> bool {
        if project_root.validate_existing_path(relative_path).is_err() {
            return false;
        }
        let Ok(target) = project_root.target_path(relative_path) else {
            return false;
        };
        let Ok(mut entries) = std::fs::read_dir(&target) else {
            return false;
        };
        if entries.next().is_some() {
            return false;
        }
        std::fs::remove_dir(&target).is_ok()
    }

    fn directory_will_be_empty(
        project_root: &ProjectRoot,
        relative_path: &Path,
        deleted_files: &BTreeSet<PathBuf>,
        removed_dirs: &BTreeSet<PathBuf>,
    ) -> bool {
        if project_root.validate_existing_path(relative_path).is_err() {
            return false;
        }
        let Ok(target) = project_root.target_path(relative_path) else {
            return false;
        };
        let Ok(entries) = std::fs::read_dir(target) else {
            return false;
        };

        for entry in entries {
            let Ok(entry) = entry else {
                return false;
            };
            let Ok(relative_entry) = entry
                .path()
                .strip_prefix(project_root.path())
                .map(Path::to_path_buf)
            else {
                return false;
            };
            let Ok(file_type) = entry.file_type() else {
                return false;
            };
            if file_type.is_dir() {
                if !removed_dirs.contains(&relative_entry) {
                    return false;
                }
            } else if !deleted_files.contains(&relative_entry) {
                return false;
            }
        }
        true
    }
}
