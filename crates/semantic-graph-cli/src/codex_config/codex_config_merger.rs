use crate::{SemanticGraphCliError, SemanticGraphCliResult, args::McpInstallMode};
use semantic_graph_agent_assets::constants::{
    generated_paths::CONFIG_SNIPPET,
    manifest::MCP_SERVER_TABLE,
    mcp::CONFIG_ROOT_TABLE,
    toml_fields::{ARGS, ENABLED},
};
use std::path::Path;
use toml::{Table, Value};

pub struct CodexConfigMerger;

impl CodexConfigMerger {
    pub fn merge(
        config_path: &Path,
        existing_config: Option<&str>,
        snippet: &str,
        mcp_mode: McpInstallMode,
        database_path: Option<&str>,
    ) -> SemanticGraphCliResult<String> {
        let mut root = match existing_config {
            Some(source) => toml::from_str::<Table>(source).map_err(|source| {
                SemanticGraphCliError::config_toml_parse(config_path.to_path_buf(), source)
            })?,
            None => Table::new(),
        };
        let mut server = Self::existing_server_table(config_path, &root)?;
        for (key, value) in Self::managed_server_from_snippet(snippet)? {
            server.insert(key, value);
        }
        server.insert(ENABLED.to_string(), Value::Boolean(mcp_mode.enabled()));
        server.insert(
            ARGS.to_string(),
            Value::Array(Self::database_path_args(database_path)),
        );

        match root.get_mut(CONFIG_ROOT_TABLE) {
            Some(Value::Table(mcp_servers)) => {
                mcp_servers.insert(MCP_SERVER_TABLE.to_string(), Value::Table(server));
            }
            Some(_) => {
                return Err(SemanticGraphCliError::invalid_install_path(
                    config_path.to_path_buf(),
                    format!("{CONFIG_ROOT_TABLE} must be a TOML table"),
                ));
            }
            None => {
                let mut mcp_servers = Table::new();
                mcp_servers.insert(MCP_SERVER_TABLE.to_string(), Value::Table(server));
                root.insert(CONFIG_ROOT_TABLE.to_string(), Value::Table(mcp_servers));
            }
        }

        let mut output =
            toml::to_string_pretty(&root).map_err(SemanticGraphCliError::config_toml_serialize)?;
        if !output.ends_with('\n') {
            output.push('\n');
        }
        Ok(output)
    }

    fn managed_server_from_snippet(snippet: &str) -> SemanticGraphCliResult<Table> {
        let value = toml::from_str::<Value>(snippet).map_err(|source| {
            SemanticGraphCliError::config_toml_parse(
                std::path::PathBuf::from(CONFIG_SNIPPET),
                source,
            )
        })?;
        let root = value
            .get(CONFIG_ROOT_TABLE)
            .and_then(Value::as_table)
            .ok_or_else(|| {
                SemanticGraphCliError::invalid_install_path(
                    std::path::PathBuf::from(CONFIG_SNIPPET),
                    format!("missing {CONFIG_ROOT_TABLE} table"),
                )
            })?;
        let server = root
            .get(MCP_SERVER_TABLE)
            .and_then(Value::as_table)
            .ok_or_else(|| {
                SemanticGraphCliError::invalid_install_path(
                    std::path::PathBuf::from(CONFIG_SNIPPET),
                    format!("missing {CONFIG_ROOT_TABLE}.{MCP_SERVER_TABLE} table"),
                )
            })?;
        Ok(server.clone())
    }

    fn existing_server_table(config_path: &Path, root: &Table) -> SemanticGraphCliResult<Table> {
        let Some(mcp_servers) = root.get(CONFIG_ROOT_TABLE) else {
            return Ok(Table::new());
        };
        let Some(mcp_servers) = mcp_servers.as_table() else {
            return Err(SemanticGraphCliError::invalid_install_path(
                config_path.to_path_buf(),
                format!("{CONFIG_ROOT_TABLE} must be a TOML table"),
            ));
        };
        let Some(server) = mcp_servers.get(MCP_SERVER_TABLE) else {
            return Ok(Table::new());
        };
        server.as_table().cloned().ok_or_else(|| {
            SemanticGraphCliError::invalid_install_path(
                config_path.to_path_buf(),
                format!("{CONFIG_ROOT_TABLE}.{MCP_SERVER_TABLE} must be a TOML table"),
            )
        })
    }

    fn database_path_args(database_path: Option<&str>) -> Vec<Value> {
        match database_path {
            Some(path) => vec![
                Value::String("--database-path".to_string()),
                Value::String(path.to_string()),
            ],
            None => Vec::new(),
        }
    }
}
