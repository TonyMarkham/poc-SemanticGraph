use crate::{
    constants::manifest::{
        DISALLOWED_EXTRACT_TOOLS_ARG, EXPECTED_ROOT_PREFIX, HOST_CODEX, MANIFEST_PATH,
        MCP_SERVER_TABLE, SKILL_NAME, SMOKE_AGENT_NAME_FRAGMENT,
    },
    error::{AgentAssetsError, AgentAssetsResult},
    manifest::{
        CustomAgentAsset, HostManifest, ManifestPaths, McpServerAsset, ReadmeAsset, ReferenceAsset,
        SkillAsset,
    },
};
use serde::Deserialize;
use std::{
    collections::BTreeSet,
    fs,
    path::{Component, Path, PathBuf},
};

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetManifest {
    pub host: HostManifest,
    pub paths: ManifestPaths,
    pub skill: SkillAsset,
    #[serde(default)]
    pub references: Vec<ReferenceAsset>,
    #[serde(default)]
    pub custom_agents: Vec<CustomAgentAsset>,
    pub mcp_server: McpServerAsset,
    pub readme: ReadmeAsset,
}

impl AssetManifest {
    pub fn load_from_repo(repo_root: &Path) -> AgentAssetsResult<Self> {
        let path = repo_root.join(MANIFEST_PATH);
        let source = fs::read_to_string(&path)
            .map_err(|source| AgentAssetsError::io("read manifest", Some(path.clone()), source))?;
        Self::from_toml_str(&source, &path)
    }

    pub fn from_toml_str(source: &str, path: &Path) -> AgentAssetsResult<Self> {
        let manifest = toml::from_str::<Self>(source)
            .map_err(|source| AgentAssetsError::manifest_toml(path.to_path_buf(), source))?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn validate(&self) -> AgentAssetsResult<()> {
        if self.host.name != HOST_CODEX {
            return Err(AgentAssetsError::invalid_manifest(format!(
                "unsupported host variant {}; expected {HOST_CODEX}",
                self.host.name
            )));
        }

        validate_relative_path(&self.paths.expected_root, "expected root")?;
        validate_relative_path(&self.paths.fragment_root, "fragment root")?;
        if !self
            .paths
            .expected_root
            .starts_with(Path::new(EXPECTED_ROOT_PREFIX))
        {
            return Err(AgentAssetsError::invalid_manifest(format!(
                "expected root {} must be under {EXPECTED_ROOT_PREFIX}",
                self.paths.expected_root.display()
            )));
        }

        if self.skill.name != SKILL_NAME {
            return Err(AgentAssetsError::invalid_manifest(format!(
                "skill name must be {SKILL_NAME}"
            )));
        }

        if self.mcp_server.table_name != MCP_SERVER_TABLE {
            return Err(AgentAssetsError::invalid_manifest(format!(
                "MCP server table name must be {MCP_SERVER_TABLE}"
            )));
        }

        if self
            .mcp_server
            .args
            .iter()
            .any(|arg| arg == DISALLOWED_EXTRACT_TOOLS_ARG)
        {
            return Err(AgentAssetsError::invalid_manifest(format!(
                "Phase 3 MCP config must not include {DISALLOWED_EXTRACT_TOOLS_ARG}"
            )));
        }

        let mut output_paths = BTreeSet::new();
        self.record_output_path(&self.skill.output_path, &mut output_paths)?;
        for reference in &self.references {
            self.record_output_path(&reference.output_path, &mut output_paths)?;
        }
        for custom_agent in &self.custom_agents {
            self.record_output_path(&custom_agent.output_path, &mut output_paths)?;
            if custom_agent.name.contains(SMOKE_AGENT_NAME_FRAGMENT) {
                return Err(AgentAssetsError::invalid_manifest(
                    "Phase 3 must not generate a smoke-test custom agent",
                ));
            }
        }
        self.record_output_path(&self.mcp_server.output_path, &mut output_paths)?;
        self.record_output_path(&self.readme.output_path, &mut output_paths)?;

        for fragment in self.all_fragment_paths() {
            validate_relative_path(fragment, "fragment")?;
        }

        Ok(())
    }

    pub fn expected_root(&self, repo_root: &Path) -> PathBuf {
        repo_root.join(&self.paths.expected_root)
    }

    pub fn fragment_root(&self, repo_root: &Path) -> PathBuf {
        repo_root.join(&self.paths.fragment_root)
    }

    pub fn declared_output_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        paths.push(self.skill.output_path.clone());
        for reference in &self.references {
            paths.push(reference.output_path.clone());
        }
        for custom_agent in &self.custom_agents {
            paths.push(custom_agent.output_path.clone());
        }
        paths.push(self.mcp_server.output_path.clone());
        paths.push(self.readme.output_path.clone());
        paths
    }

    pub fn all_fragment_paths(&self) -> Vec<&PathBuf> {
        let mut paths = Vec::new();
        for fragment in &self.skill.fragments {
            paths.push(fragment);
        }
        for reference in &self.references {
            for fragment in &reference.fragments {
                paths.push(fragment);
            }
        }
        for custom_agent in &self.custom_agents {
            for fragment in &custom_agent.fragments {
                paths.push(fragment);
            }
        }
        for fragment in &self.readme.fragments {
            paths.push(fragment);
        }
        paths
    }

    fn record_output_path(
        &self,
        output_path: &Path,
        output_paths: &mut BTreeSet<PathBuf>,
    ) -> AgentAssetsResult<()> {
        validate_relative_path(output_path, "output path").map_err(|error| match error {
            AgentAssetsError::InvalidManifest { .. } => {
                AgentAssetsError::output_path_escapes_expected_root(
                    output_path.to_path_buf(),
                    self.paths.expected_root.clone(),
                )
            }
            other => other,
        })?;

        if !output_paths.insert(output_path.to_path_buf()) {
            return Err(AgentAssetsError::duplicate_output_path(
                output_path.to_path_buf(),
            ));
        }

        Ok(())
    }
}

fn validate_relative_path(path: &Path, label: &str) -> AgentAssetsResult<()> {
    if path.as_os_str().is_empty() {
        return Err(AgentAssetsError::invalid_manifest(format!(
            "{label} must not be empty"
        )));
    }

    for component in path.components() {
        match component {
            Component::Normal(_) | Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(AgentAssetsError::invalid_manifest(format!(
                    "{label} {} must be relative and must not contain ..",
                    path.display()
                )));
            }
        }
    }

    Ok(())
}
