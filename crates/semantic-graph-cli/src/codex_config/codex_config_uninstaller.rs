use crate::{SemanticGraphCliError, SemanticGraphCliResult};
use semantic_graph_agent_assets::constants::{
    manifest::MCP_SERVER_TABLE,
    mcp::CONFIG_ROOT_TABLE,
    toml_fields::{ARGS, COMMAND, ENABLED, REQUIRED},
};
use std::path::Path;
use toml::{Table, Value};

pub struct CodexConfigUninstaller;

impl CodexConfigUninstaller {
    pub fn uninstall(config_path: &Path, source: &str) -> SemanticGraphCliResult<Option<String>> {
        let mut root = toml::from_str::<Table>(source).map_err(|source| {
            SemanticGraphCliError::config_toml_parse(config_path.to_path_buf(), source)
        })?;
        let mut changed = false;

        if let Some(mcp_servers) = root.get_mut(CONFIG_ROOT_TABLE) {
            let mcp_servers = mcp_servers.as_table_mut().ok_or_else(|| {
                SemanticGraphCliError::invalid_install_path(
                    config_path.to_path_buf(),
                    format!("{CONFIG_ROOT_TABLE} must be a TOML table"),
                )
            })?;

            if let Some(server) = mcp_servers.get_mut(MCP_SERVER_TABLE) {
                let server = server.as_table_mut().ok_or_else(|| {
                    SemanticGraphCliError::invalid_install_path(
                        config_path.to_path_buf(),
                        format!("{CONFIG_ROOT_TABLE}.{MCP_SERVER_TABLE} must be a TOML table"),
                    )
                })?;
                for key in Self::managed_keys() {
                    if server.remove(*key).is_some() {
                        changed = true;
                    }
                }
                if server.is_empty() {
                    mcp_servers.remove(MCP_SERVER_TABLE);
                    changed = true;
                }
            }

            if mcp_servers.is_empty() {
                root.remove(CONFIG_ROOT_TABLE);
                changed = true;
            }
        }

        if root.is_empty() {
            return Ok(None);
        }
        if !changed {
            return Ok(Some(source.to_string()));
        }

        let output = toml::to_string_pretty(&Value::Table(root))
            .map_err(SemanticGraphCliError::config_toml_serialize)?;
        Ok(Some(output))
    }

    fn managed_keys() -> &'static [&'static str] {
        &[
            COMMAND,
            ARGS,
            ENABLED,
            REQUIRED,
            "startup_timeout_sec",
            "tool_timeout_sec",
        ]
    }
}
