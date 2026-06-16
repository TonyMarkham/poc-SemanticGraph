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
        let root = match existing_config {
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

        let managed_block = Self::managed_server_block(server)?;
        let output = match existing_config {
            Some(source) => Self::merge_managed_block(config_path, source, &managed_block)?,
            None => managed_block,
        };
        toml::from_str::<Table>(&output).map_err(|source| {
            SemanticGraphCliError::config_toml_parse(config_path.to_path_buf(), source)
        })?;
        Ok(output)
    }

    fn managed_server_block(server: Table) -> SemanticGraphCliResult<String> {
        let mut semantic_graph = Table::new();
        semantic_graph.insert(MCP_SERVER_TABLE.to_string(), Value::Table(server));
        let mut mcp_servers = Table::new();
        mcp_servers.insert(CONFIG_ROOT_TABLE.to_string(), Value::Table(semantic_graph));
        let mut output = toml::to_string_pretty(&mcp_servers)
            .map_err(SemanticGraphCliError::config_toml_serialize)?;
        if !output.ends_with('\n') {
            output.push('\n');
        }
        Ok(output)
    }

    fn merge_managed_block(
        config_path: &Path,
        source: &str,
        managed_block: &str,
    ) -> SemanticGraphCliResult<String> {
        if let Some((start, end)) = Self::find_table_range(source, "mcp_servers.semantic_graph") {
            let mut output = String::new();
            output.push_str(&source[..start]);
            output.push_str(managed_block);
            output.push_str(&source[end..]);
            return Ok(output);
        }

        let root = toml::from_str::<Table>(source).map_err(|source| {
            SemanticGraphCliError::config_toml_parse(config_path.to_path_buf(), source)
        })?;
        if matches!(root.get(CONFIG_ROOT_TABLE), Some(value) if !value.is_table()) {
            return Err(SemanticGraphCliError::invalid_install_path(
                config_path.to_path_buf(),
                format!("{CONFIG_ROOT_TABLE} must be a TOML table"),
            ));
        }

        let mut output = source.to_string();
        if !output.ends_with('\n') {
            output.push('\n');
        }
        if !output.ends_with("\n\n") {
            output.push('\n');
        }
        output.push_str(managed_block);
        Ok(output)
    }

    fn find_table_range(source: &str, target_table: &str) -> Option<(usize, usize)> {
        let mut found_start = None;
        let mut offset = 0;

        for line in source.split_inclusive('\n') {
            if Self::is_toml_table_header(line) {
                if found_start.is_some() {
                    return found_start.map(|start| (start, offset));
                }
                if Self::table_header_name(line).as_deref() == Some(target_table) {
                    found_start = Some(offset);
                }
            }
            offset += line.len();
        }

        found_start.map(|start| (start, source.len()))
    }

    fn is_toml_table_header(line: &str) -> bool {
        let trimmed = line.trim();
        trimmed.starts_with('[') && trimmed.ends_with(']')
    }

    fn table_header_name(line: &str) -> Option<String> {
        let trimmed = line.trim();
        if trimmed.starts_with("[[") && trimmed.ends_with("]]") {
            return Some(trimmed[2..trimmed.len() - 2].trim().to_string());
        }
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            return Some(trimmed[1..trimmed.len() - 1].trim().to_string());
        }
        None
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
