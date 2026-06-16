use crate::install::{FileAction, InstallManifest, ManagedFile, ProjectRoot};

#[derive(Clone, Debug)]
pub struct CodexUninstallPlan {
    project_root: ProjectRoot,
    manifest: InstallManifest,
    actions: Vec<FileAction>,
    config_file: Option<ManagedFile>,
}

impl CodexUninstallPlan {
    pub fn new(
        project_root: ProjectRoot,
        manifest: InstallManifest,
        actions: Vec<FileAction>,
        config_file: Option<ManagedFile>,
    ) -> Self {
        Self {
            project_root,
            manifest,
            actions,
            config_file,
        }
    }

    pub fn project_root(&self) -> &ProjectRoot {
        &self.project_root
    }

    pub fn manifest(&self) -> &InstallManifest {
        &self.manifest
    }

    pub fn actions(&self) -> &[FileAction] {
        &self.actions
    }

    pub fn config_file(&self) -> Option<&ManagedFile> {
        self.config_file.as_ref()
    }
}
