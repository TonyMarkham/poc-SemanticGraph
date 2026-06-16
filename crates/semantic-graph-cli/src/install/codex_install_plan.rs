use crate::install::{ManagedFile, ProjectRoot};

#[derive(Clone, Debug)]
pub struct CodexInstallPlan {
    project_root: ProjectRoot,
    managed_files: Vec<ManagedFile>,
}

impl CodexInstallPlan {
    pub fn new(project_root: ProjectRoot, managed_files: Vec<ManagedFile>) -> Self {
        Self {
            project_root,
            managed_files,
        }
    }

    pub fn project_root(&self) -> &ProjectRoot {
        &self.project_root
    }

    pub fn managed_files(&self) -> &[ManagedFile] {
        &self.managed_files
    }
}
