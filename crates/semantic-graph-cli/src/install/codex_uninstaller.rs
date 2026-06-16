use crate::{
    SemanticGraphCliError, SemanticGraphCliResult,
    args::CodexUninstallArgs,
    codex_config::CodexConfigUninstaller,
    constants::codex_paths::{CONFIG, INSTALL_MANIFEST},
    install::{
        AtomicFileWriter, Checksum, CodexUninstallPlan, CodexUninstallReport, DirectoryCleanup,
        FileAction, FileActionKind, ManagedFile, ManagedFileManifestEntry, ManifestWriter,
        ProjectRoot,
    },
};
use std::path::{Path, PathBuf};

pub struct CodexUninstaller;

impl CodexUninstaller {
    pub fn uninstall(
        args: &CodexUninstallArgs,
        current_dir: &Path,
    ) -> SemanticGraphCliResult<CodexUninstallReport> {
        let project_root = ProjectRoot::resolve(args.project(), current_dir)?;
        let plan = Self::build_plan(project_root, args.force())?;
        let report = CodexUninstallReport::new(
            plan.project_root().path().to_path_buf(),
            args.dry_run(),
            plan.actions().to_vec(),
        );

        if report.has_refusals() {
            return Err(SemanticGraphCliError::refused_uninstall(report));
        }

        if args.dry_run() {
            return Ok(report);
        }

        Self::apply_plan(&plan)?;
        Ok(report)
    }

    fn build_plan(
        project_root: ProjectRoot,
        force: bool,
    ) -> SemanticGraphCliResult<CodexUninstallPlan> {
        let manifest = ManifestWriter::load_required(&project_root)?;
        let mut actions = Vec::new();
        let mut config_file = None;

        for entry in &manifest.managed_files {
            if entry.path == CONFIG {
                let (action, planned_config_file) = Self::plan_config_action(&project_root, entry)?;
                actions.push(action);
                config_file = planned_config_file;
            } else {
                actions.push(Self::plan_managed_file_delete(&project_root, entry, force)?);
            }
        }

        actions.push(FileAction::new(
            FileActionKind::Delete,
            ManifestWriter::manifest_relative_path(),
        ));
        actions.extend(DirectoryCleanup::plan(&project_root, &actions));

        Ok(CodexUninstallPlan::new(
            project_root,
            manifest,
            actions,
            config_file,
        ))
    }

    fn plan_managed_file_delete(
        project_root: &ProjectRoot,
        entry: &ManagedFileManifestEntry,
        force: bool,
    ) -> SemanticGraphCliResult<FileAction> {
        let relative_path = PathBuf::from(&entry.path);
        project_root.validate_existing_path(&relative_path)?;
        let target = project_root.target_path(&relative_path)?;
        match std::fs::read(&target) {
            Ok(existing) if force || entry.sha256 == Checksum::sha256_hex(&existing) => {
                Ok(FileAction::new(FileActionKind::Delete, relative_path))
            }
            Ok(_) => Ok(FileAction::refused(
                relative_path,
                "checksum mismatch; use --force to delete this manifest-managed file",
            )),
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
                Ok(FileAction::new(FileActionKind::Missing, relative_path))
            }
            Err(source) => Err(SemanticGraphCliError::io(
                "read uninstall target",
                Some(target),
                source,
            )),
        }
    }

    fn plan_config_action(
        project_root: &ProjectRoot,
        entry: &ManagedFileManifestEntry,
    ) -> SemanticGraphCliResult<(FileAction, Option<ManagedFile>)> {
        let relative_path = PathBuf::from(&entry.path);
        project_root.validate_existing_path(&relative_path)?;
        let target = project_root.target_path(&relative_path)?;
        match std::fs::read_to_string(&target) {
            Ok(source) => match CodexConfigUninstaller::uninstall(&target, &source) {
                Ok(None) => Ok((FileAction::new(FileActionKind::Delete, relative_path), None)),
                Ok(Some(content)) if content == source => {
                    Ok((FileAction::new(FileActionKind::Skip, relative_path), None))
                }
                Ok(Some(content)) => Ok((
                    FileAction::new(FileActionKind::Update, relative_path.clone()),
                    Some(ManagedFile::new(relative_path, content)),
                )),
                Err(error) if Self::is_config_refusal(&error) => Ok((
                    FileAction::refused(relative_path, error.user_message()),
                    None,
                )),
                Err(error) => Err(error),
            },
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok((
                FileAction::new(FileActionKind::Missing, relative_path),
                None,
            )),
            Err(source) => Err(SemanticGraphCliError::io(
                "read Codex config",
                Some(target),
                source,
            )),
        }
    }

    fn is_config_refusal(error: &SemanticGraphCliError) -> bool {
        matches!(
            error,
            SemanticGraphCliError::ConfigTomlParse { .. }
                | SemanticGraphCliError::InvalidInstallPath { .. }
        )
    }

    fn apply_plan(plan: &CodexUninstallPlan) -> SemanticGraphCliResult<()> {
        for action in plan.actions() {
            if action.kind() == FileActionKind::RemoveDir {
                continue;
            }
            if action.relative_path() == Path::new(INSTALL_MANIFEST) {
                continue;
            }
            if action.relative_path() == Path::new(CONFIG) {
                Self::apply_config_action(plan, action)?;
                continue;
            }
            if action.kind().deletes_file() {
                Self::remove_file(plan.project_root(), action.relative_path())?;
            }
        }

        if Self::action_for(plan.actions(), Path::new(INSTALL_MANIFEST)).deletes_file() {
            Self::remove_file(plan.project_root(), Path::new(INSTALL_MANIFEST))?;
        }
        Ok(())
    }

    fn apply_config_action(
        plan: &CodexUninstallPlan,
        action: &FileAction,
    ) -> SemanticGraphCliResult<()> {
        match action.kind() {
            FileActionKind::Update => {
                let config_file = plan.config_file().ok_or_else(|| {
                    SemanticGraphCliError::invalid_install_path(
                        action.relative_path().to_path_buf(),
                        "config update was planned without replacement content",
                    )
                })?;
                AtomicFileWriter::write(
                    plan.project_root(),
                    config_file.relative_path(),
                    config_file.content(),
                )
            }
            FileActionKind::Delete => {
                Self::remove_file(plan.project_root(), action.relative_path())
            }
            _ => Ok(()),
        }
    }

    fn remove_file(project_root: &ProjectRoot, relative_path: &Path) -> SemanticGraphCliResult<()> {
        project_root.validate_existing_path(relative_path)?;
        let target = project_root.target_path(relative_path)?;
        match std::fs::remove_file(&target) {
            Ok(()) => {
                DirectoryCleanup::cleanup_after_file_delete(project_root, relative_path);
                Ok(())
            }
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(source) => Err(SemanticGraphCliError::io(
                "delete uninstall target",
                Some(target),
                source,
            )),
        }
    }

    fn action_for(actions: &[FileAction], relative_path: &Path) -> FileActionKind {
        actions
            .iter()
            .find(|action| action.relative_path() == relative_path)
            .map(FileAction::kind)
            .unwrap_or(FileActionKind::Skip)
    }
}
