use crate::{
    constants::{
        mcp::CONFIG_SNIPPET_ARTIFACT,
        render::{PROGRESSIVE_REFERENCES_HEADING, README_TITLE, REFERENCES_DIR, SKILL_TITLE},
    },
    error::{AgentAssetsError, AgentAssetsResult},
    manifest::{AssetManifest, CustomAgentAsset, McpServerAsset, ReferenceAsset},
    render::{CustomAgentToml, McpConfigToml, McpServerToml, RenderedAsset},
};
use std::{collections::BTreeMap, fs, path::Path};

pub struct AssetRenderer;

impl AssetRenderer {
    pub fn render(
        repo_root: &Path,
        manifest: &AssetManifest,
    ) -> AgentAssetsResult<Vec<RenderedAsset>> {
        manifest.validate()?;

        let mut assets = Vec::new();
        assets.push(Self::render_skill(repo_root, manifest)?);

        for reference in &manifest.references {
            assets.push(Self::render_reference(repo_root, manifest, reference)?);
        }

        for custom_agent in &manifest.custom_agents {
            assets.push(Self::render_custom_agent(
                repo_root,
                manifest,
                custom_agent,
            )?);
        }

        assets.push(Self::render_mcp_config(manifest, &manifest.mcp_server)?);
        assets.push(Self::render_readme(repo_root, manifest)?);

        Ok(assets)
    }

    fn render_skill(
        repo_root: &Path,
        manifest: &AssetManifest,
    ) -> AgentAssetsResult<RenderedAsset> {
        let body = Self::load_fragments(repo_root, manifest, &manifest.skill.fragments)?;
        let mut content = String::new();
        content.push_str("---\n");
        content.push_str("name: ");
        content.push_str(&manifest.skill.name);
        content.push('\n');
        content.push_str("description: \"");
        content.push_str(&escape_frontmatter(&manifest.skill.description));
        content.push_str("\"\n");
        content.push_str("---\n\n");
        content.push_str("# ");
        content.push_str(SKILL_TITLE);
        content.push_str("\n\n");
        content.push_str(&body);
        content.push_str("\n\n## ");
        content.push_str(PROGRESSIVE_REFERENCES_HEADING);
        content.push_str("\n\n");
        for reference in &manifest.references {
            content.push_str("- `");
            content.push_str(&reference_file_name(reference)?);
            content.push_str("` - ");
            content.push_str(&reference.title);
            content.push('\n');
        }

        Ok(RenderedAsset::new(
            manifest.skill.output_path.clone(),
            content,
        ))
    }

    fn render_reference(
        repo_root: &Path,
        manifest: &AssetManifest,
        reference: &ReferenceAsset,
    ) -> AgentAssetsResult<RenderedAsset> {
        let body = Self::load_fragments(repo_root, manifest, &reference.fragments)?;
        let content = format!("# {}\n\n{}", reference.title, body);
        Ok(RenderedAsset::new(reference.output_path.clone(), content))
    }

    fn render_custom_agent(
        repo_root: &Path,
        manifest: &AssetManifest,
        custom_agent: &CustomAgentAsset,
    ) -> AgentAssetsResult<RenderedAsset> {
        let developer_instructions =
            Self::load_fragments(repo_root, manifest, &custom_agent.fragments)?;
        let toml = toml::to_string_pretty(&CustomAgentToml {
            name: &custom_agent.name,
            description: &custom_agent.description,
            developer_instructions: &developer_instructions,
        })
        .map_err(|source| AgentAssetsError::toml_serialize(&custom_agent.name, source))?;

        Ok(RenderedAsset::new(custom_agent.output_path.clone(), toml))
    }

    fn render_mcp_config(
        manifest: &AssetManifest,
        mcp_server: &McpServerAsset,
    ) -> AgentAssetsResult<RenderedAsset> {
        let mut mcp_servers = BTreeMap::new();
        mcp_servers.insert(
            mcp_server.table_name.as_str(),
            McpServerToml {
                command: &mcp_server.command,
                args: &mcp_server.args,
                enabled: mcp_server.enabled,
                required: mcp_server.required,
                startup_timeout_sec: mcp_server.startup_timeout_sec,
                tool_timeout_sec: mcp_server.tool_timeout_sec,
            },
        );

        let toml = toml::to_string_pretty(&McpConfigToml { mcp_servers })
            .map_err(|source| AgentAssetsError::toml_serialize(CONFIG_SNIPPET_ARTIFACT, source))?;

        Ok(RenderedAsset::new(
            manifest.mcp_server.output_path.clone(),
            toml,
        ))
    }

    fn render_readme(
        repo_root: &Path,
        manifest: &AssetManifest,
    ) -> AgentAssetsResult<RenderedAsset> {
        let body = Self::load_fragments(repo_root, manifest, &manifest.readme.fragments)?;
        let content = format!("# {README_TITLE}\n\n{body}");
        Ok(RenderedAsset::new(
            manifest.readme.output_path.clone(),
            content,
        ))
    }

    fn load_fragments(
        repo_root: &Path,
        manifest: &AssetManifest,
        fragments: &[std::path::PathBuf],
    ) -> AgentAssetsResult<String> {
        let fragment_root = manifest.fragment_root(repo_root);
        let mut sections = Vec::new();

        for fragment in fragments {
            let path = fragment_root.join(fragment);
            match fs::read_to_string(&path) {
                Ok(text) => sections.push(text.trim().to_string()),
                Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
                    return Err(AgentAssetsError::missing_fragment(path));
                }
                Err(source) => {
                    return Err(AgentAssetsError::io("read fragment", Some(path), source));
                }
            }
        }

        Ok(sections.join("\n\n"))
    }
}

fn escape_frontmatter(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn reference_file_name(reference: &ReferenceAsset) -> AgentAssetsResult<String> {
    let file_name = reference
        .output_path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            AgentAssetsError::invalid_manifest(format!(
                "reference {} output path {} has no UTF-8 file name",
                reference.name,
                reference.output_path.display()
            ))
        })?;

    Ok(format!("{REFERENCES_DIR}/{file_name}"))
}
