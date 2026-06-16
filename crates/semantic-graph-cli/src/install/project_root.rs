use crate::{SemanticGraphCliError, SemanticGraphCliResult, install::PathValidator};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectRoot {
    path: PathBuf,
}

impl ProjectRoot {
    pub fn resolve(project: &Path, current_dir: &Path) -> SemanticGraphCliResult<Self> {
        let candidate = if project.is_absolute() {
            project.to_path_buf()
        } else {
            current_dir.join(project)
        };
        let path = std::fs::canonicalize(&candidate).map_err(|source| {
            SemanticGraphCliError::io("canonicalize project", Some(candidate.clone()), source)
        })?;
        let metadata = std::fs::metadata(&path).map_err(|source| {
            SemanticGraphCliError::io("inspect project", Some(path.clone()), source)
        })?;
        if !metadata.is_dir() {
            return Err(SemanticGraphCliError::invalid_project(format!(
                "{} is not a directory",
                path.display()
            )));
        }
        Ok(Self { path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn target_path(&self, relative_path: &Path) -> SemanticGraphCliResult<PathBuf> {
        PathValidator::validate_project_relative(relative_path)?;
        Ok(self.path.join(relative_path))
    }

    pub fn validate_existing_path(&self, relative_path: &Path) -> SemanticGraphCliResult<()> {
        PathValidator::validate_existing_path(&self.path, relative_path)
    }

    pub fn ensure_parent_dir(&self, relative_path: &Path) -> SemanticGraphCliResult<()> {
        self.validate_existing_path(relative_path)?;
        let target = self.target_path(relative_path)?;
        let parent = target.parent().ok_or_else(|| {
            SemanticGraphCliError::invalid_install_path(
                relative_path.to_path_buf(),
                "target path has no parent directory",
            )
        })?;
        std::fs::create_dir_all(parent).map_err(|source| {
            SemanticGraphCliError::io(
                "create parent directories",
                Some(parent.to_path_buf()),
                source,
            )
        })
    }
}
