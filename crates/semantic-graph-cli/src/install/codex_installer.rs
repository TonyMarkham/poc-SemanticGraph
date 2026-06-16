use crate::{
    SemanticGraphCliError, SemanticGraphCliResult,
    args::CodexInstallArgs,
    codex_config::CodexConfigMerger,
    constants::codex_paths::CONFIG,
    install::{
        AtomicFileWriter, Checksum, CodexInstallPlan, CodexInstallReport, FileAction,
        FileActionKind, InstallManifest, ManagedFile, ManifestWriter, ProjectRoot,
    },
};
use semantic_graph_agent_assets::{
    AssetManifest, AssetRenderer, constants::generated_paths::CONFIG_SNIPPET,
};
use std::path::{Path, PathBuf};

pub struct CodexInstaller {
    repo_root: PathBuf,
}

impl CodexInstaller {
    pub fn new(repo_root: PathBuf) -> Self {
        Self { repo_root }
    }

    pub fn install(
        &self,
        args: &CodexInstallArgs,
        current_dir: &Path,
    ) -> SemanticGraphCliResult<CodexInstallReport> {
        let project_root = ProjectRoot::resolve(args.project(), current_dir)?;
        let plan = self.build_plan(args, project_root)?;
        let existing_manifest = ManifestWriter::load_existing(plan.project_root())?;
        let mut actions = self.plan_actions(&plan, existing_manifest.as_ref(), args.force())?;
        let manifest_file =
            self.build_manifest_file(args, plan.project_root(), plan.managed_files(), &actions)?;
        let manifest_action = self.plan_manifest_action(
            plan.project_root(),
            &manifest_file,
            existing_manifest.as_ref(),
            args.force(),
        )?;
        actions.push(manifest_action);

        let report = CodexInstallReport::new(
            plan.project_root().path().to_path_buf(),
            args.dry_run(),
            args.mcp(),
            actions.clone(),
        );

        if report.has_refusals() {
            return Err(SemanticGraphCliError::refused_writes(report));
        }

        if args.dry_run() {
            return Ok(report);
        }

        self.write_files(
            plan.project_root(),
            plan.managed_files(),
            &manifest_file,
            &actions,
        )?;
        Ok(report)
    }

    fn build_plan(
        &self,
        args: &CodexInstallArgs,
        project_root: ProjectRoot,
    ) -> SemanticGraphCliResult<CodexInstallPlan> {
        let manifest = AssetManifest::load_from_repo(&self.repo_root)
            .map_err(SemanticGraphCliError::agent_assets)?;
        let assets = AssetRenderer::render(&self.repo_root, &manifest)
            .map_err(SemanticGraphCliError::agent_assets)?;
        let snippet = assets
            .iter()
            .find(|asset| asset.output_path() == Path::new(CONFIG_SNIPPET))
            .map(|asset| asset.content().to_string())
            .ok_or_else(|| {
                SemanticGraphCliError::invalid_install_path(
                    PathBuf::from(CONFIG_SNIPPET),
                    "rendered Codex assets did not include MCP config snippet",
                )
            })?;

        let mut managed_files = Vec::new();
        for asset in assets {
            if asset.output_path() == Path::new(CONFIG_SNIPPET) {
                continue;
            }
            project_root.target_path(asset.output_path())?;
            managed_files.push(ManagedFile::new(
                asset.output_path().clone(),
                asset.content().to_string(),
            ));
        }

        let config_relative_path = PathBuf::from(CONFIG);
        let config_target_path = project_root.target_path(&config_relative_path)?;
        project_root.validate_existing_path(&config_relative_path)?;
        let existing_config = match std::fs::read_to_string(&config_target_path) {
            Ok(source) => Some(source),
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => None,
            Err(source) => {
                return Err(SemanticGraphCliError::io(
                    "read Codex config",
                    Some(config_target_path),
                    source,
                ));
            }
        };
        let config_content = CodexConfigMerger::merge(
            &config_target_path,
            existing_config.as_deref(),
            &snippet,
            args.mcp(),
            args.database_path(),
        )?;
        managed_files.push(ManagedFile::new(config_relative_path, config_content));
        managed_files.sort_by(|left, right| left.relative_path().cmp(right.relative_path()));

        Ok(CodexInstallPlan::new(project_root, managed_files))
    }

    fn plan_actions(
        &self,
        plan: &CodexInstallPlan,
        existing_manifest: Option<&InstallManifest>,
        force: bool,
    ) -> SemanticGraphCliResult<Vec<FileAction>> {
        let mut actions = Vec::new();
        for file in plan.managed_files() {
            let action =
                self.plan_file_action(plan.project_root(), file, existing_manifest, force, false)?;
            actions.push(action);
        }
        Ok(actions)
    }

    fn build_manifest_file(
        &self,
        args: &CodexInstallArgs,
        project_root: &ProjectRoot,
        managed_files: &[ManagedFile],
        actions: &[FileAction],
    ) -> SemanticGraphCliResult<ManagedFile> {
        let manifest = ManifestWriter::build(
            project_root,
            args.mcp(),
            args.database_path(),
            managed_files,
            actions,
        );
        let content = ManifestWriter::serialize(&manifest)?;
        Ok(ManagedFile::new(
            ManifestWriter::manifest_relative_path(),
            content,
        ))
    }

    fn plan_manifest_action(
        &self,
        project_root: &ProjectRoot,
        manifest_file: &ManagedFile,
        existing_manifest: Option<&InstallManifest>,
        force: bool,
    ) -> SemanticGraphCliResult<FileAction> {
        self.plan_file_action(project_root, manifest_file, existing_manifest, force, true)
    }

    fn plan_file_action(
        &self,
        project_root: &ProjectRoot,
        file: &ManagedFile,
        existing_manifest: Option<&InstallManifest>,
        force: bool,
        is_manifest_file: bool,
    ) -> SemanticGraphCliResult<FileAction> {
        project_root.validate_existing_path(file.relative_path())?;
        let target = project_root.target_path(file.relative_path())?;
        match std::fs::read(&target) {
            Ok(existing) if existing == file.bytes() => Ok(FileAction::new(
                FileActionKind::Skip,
                file.relative_path().to_path_buf(),
            )),
            Ok(existing)
                if self.can_update_existing(
                    file,
                    &existing,
                    existing_manifest,
                    force,
                    is_manifest_file,
                ) =>
            {
                Ok(FileAction::new(
                    FileActionKind::Update,
                    file.relative_path().to_path_buf(),
                ))
            }
            Ok(_) => Ok(FileAction::new(
                FileActionKind::Refuse,
                file.relative_path().to_path_buf(),
            )),
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(FileAction::new(
                FileActionKind::Create,
                file.relative_path().to_path_buf(),
            )),
            Err(source) => Err(SemanticGraphCliError::io(
                "read install target",
                Some(target),
                source,
            )),
        }
    }

    fn can_update_existing(
        &self,
        file: &ManagedFile,
        existing: &[u8],
        existing_manifest: Option<&InstallManifest>,
        force: bool,
        is_manifest_file: bool,
    ) -> bool {
        force
            || file.relative_path() == Path::new(CONFIG)
            || is_manifest_file && existing_manifest.is_some()
            || existing_manifest
                .and_then(|manifest| manifest.checksum_for_path(file.relative_path()))
                .map(|checksum| checksum == Checksum::sha256_hex(existing))
                .unwrap_or(false)
    }

    fn write_files(
        &self,
        project_root: &ProjectRoot,
        managed_files: &[ManagedFile],
        manifest_file: &ManagedFile,
        actions: &[FileAction],
    ) -> SemanticGraphCliResult<()> {
        for file in managed_files {
            if self.action_for(actions, file.relative_path()).writes_file() {
                AtomicFileWriter::write(project_root, file.relative_path(), file.content())?;
            }
        }
        if self
            .action_for(actions, manifest_file.relative_path())
            .writes_file()
        {
            AtomicFileWriter::write(
                project_root,
                manifest_file.relative_path(),
                manifest_file.content(),
            )?;
        }
        Ok(())
    }

    fn action_for(&self, actions: &[FileAction], relative_path: &Path) -> FileActionKind {
        actions
            .iter()
            .find(|action| action.relative_path() == relative_path)
            .map(FileAction::kind)
            .unwrap_or(FileActionKind::Skip)
    }
}
