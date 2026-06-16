use crate::{SemanticGraphCliError, SemanticGraphCliResult};
use std::path::{Component, Path, PathBuf};

pub struct PathValidator;

impl PathValidator {
    pub fn validate_project_relative(path: &Path) -> SemanticGraphCliResult<()> {
        if path.as_os_str().is_empty() {
            return Err(SemanticGraphCliError::invalid_install_path(
                path.to_path_buf(),
                "path must not be empty",
            ));
        }
        let mut has_normal_component = false;
        for component in path.components() {
            match component {
                Component::Normal(_) => {
                    has_normal_component = true;
                }
                Component::CurDir => {}
                Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                    return Err(SemanticGraphCliError::invalid_install_path(
                        path.to_path_buf(),
                        "path must be project-relative and must not contain ..",
                    ));
                }
            }
        }
        if !has_normal_component {
            return Err(SemanticGraphCliError::invalid_install_path(
                path.to_path_buf(),
                "path must include a file name",
            ));
        }
        Ok(())
    }

    pub fn validate_existing_path(root: &Path, relative_path: &Path) -> SemanticGraphCliResult<()> {
        Self::validate_project_relative(relative_path)?;
        let mut current = PathBuf::from(root);
        for component in relative_path.components() {
            if matches!(component, Component::CurDir) {
                continue;
            }
            current.push(component.as_os_str());
            match std::fs::symlink_metadata(&current) {
                Ok(metadata) if metadata.file_type().is_symlink() => {
                    let resolved = std::fs::canonicalize(&current).map_err(|source| {
                        SemanticGraphCliError::io(
                            "canonicalize symlinked install path",
                            Some(current.clone()),
                            source,
                        )
                    })?;
                    if !resolved.starts_with(root) {
                        return Err(SemanticGraphCliError::invalid_install_path(
                            relative_path.to_path_buf(),
                            format!(
                                "symlink target escapes project root: {}",
                                resolved.display()
                            ),
                        ));
                    }
                }
                Ok(_) => {}
                Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
                    return Ok(());
                }
                Err(source) => {
                    return Err(SemanticGraphCliError::io(
                        "inspect install path",
                        Some(current),
                        source,
                    ));
                }
            }
        }
        Ok(())
    }
}
