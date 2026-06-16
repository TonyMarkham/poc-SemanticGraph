use crate::{
    SemanticGraphCliError, SemanticGraphCliResult,
    args::McpInstallMode,
    constants::{
        codex_paths::INSTALL_MANIFEST,
        manifest::{ASSET_GENERATION, INSTALLER_CRATE, SCHEMA_VERSION},
    },
    install::{
        AssetSource, Checksum, FileAction, InstallManifest, InstallManifestMode, ManagedFile,
        ManagedFileManifestEntry, ProjectRoot,
    },
};
use semantic_graph_agent_assets::constants::manifest::MANIFEST_PATH;
use std::path::{Path, PathBuf};

pub struct ManifestWriter;

impl ManifestWriter {
    pub fn build(
        project_root: &ProjectRoot,
        mcp_mode: McpInstallMode,
        database_path: Option<&str>,
        managed_files: &[ManagedFile],
        actions: &[FileAction],
    ) -> InstallManifest {
        let mut entries = managed_files
            .iter()
            .map(|file| {
                let action = actions
                    .iter()
                    .find(|action| action.relative_path() == file.relative_path())
                    .map(FileAction::kind)
                    .unwrap_or(crate::install::FileActionKind::Skip);
                ManagedFileManifestEntry::new(
                    file.relative_path().to_string_lossy().to_string(),
                    Checksum::sha256_hex(file.bytes()),
                    action,
                )
            })
            .collect::<Vec<_>>();
        entries.sort_by(|left, right| left.path.cmp(&right.path));

        InstallManifest {
            schema_version: SCHEMA_VERSION,
            installer_crate: INSTALLER_CRATE.to_string(),
            installer_version: env!("CARGO_PKG_VERSION").to_string(),
            project_root: project_root.path().display().to_string(),
            mode: InstallManifestMode::new(mcp_mode, database_path.map(ToString::to_string)),
            asset_source: AssetSource::new(MANIFEST_PATH, ASSET_GENERATION),
            managed_files: entries,
        }
    }

    pub fn serialize(manifest: &InstallManifest) -> SemanticGraphCliResult<String> {
        let mut output = serde_json::to_string_pretty(manifest)
            .map_err(SemanticGraphCliError::manifest_serialize)?;
        output.push('\n');
        Ok(output)
    }

    pub fn load_existing(
        project_root: &ProjectRoot,
    ) -> SemanticGraphCliResult<Option<InstallManifest>> {
        let path = project_root.target_path(Path::new(INSTALL_MANIFEST))?;
        match std::fs::read_to_string(&path) {
            Ok(source) => serde_json::from_str::<InstallManifest>(&source)
                .map(Some)
                .map_err(|source| SemanticGraphCliError::manifest_parse(path, source)),
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(source) => Err(SemanticGraphCliError::io(
                "read install manifest",
                Some(path),
                source,
            )),
        }
    }

    pub fn manifest_relative_path() -> PathBuf {
        PathBuf::from(INSTALL_MANIFEST)
    }
}
