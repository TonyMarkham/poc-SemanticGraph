use crate::{McpServerError, McpServerResult, args::ResolvedServerConfig};

use clap::Parser;
use semantic_graph_config::{
    LoadOptions, QueryServiceConfig, discover_config, load_config, resolve_database_path,
};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Parser, PartialEq, Eq)]
#[command(name = "semantic-graph-mcp-server")]
#[command(about = "Read-only stdio MCP server for SemanticGraph SQLite stores")]
pub struct ServerArgs {
    #[arg(long = "database-path", value_name = "PATH")]
    database_path: Option<PathBuf>,
}

impl ServerArgs {
    pub fn database_path(&self) -> Option<&PathBuf> {
        self.database_path.as_ref()
    }
}

pub fn resolve_server_config(args: &ServerArgs) -> McpServerResult<ResolvedServerConfig> {
    let current_dir = std::env::current_dir()
        .map_err(|source| McpServerError::io("read current directory", None, source))?;
    resolve_server_config_from_start_dir(args, &current_dir)
}

pub fn resolve_server_config_from_start_dir(
    args: &ServerArgs,
    start_dir: impl AsRef<Path>,
) -> McpServerResult<ResolvedServerConfig> {
    if let Some(database_path) = args.database_path() {
        let resolved = resolve_database_path(LoadOptions {
            explicit_database_path: Some(database_path.clone()),
            ..LoadOptions::default()
        })
        .map_err(McpServerError::config)?;

        return Ok(ResolvedServerConfig::new(
            resolved.path().to_path_buf(),
            resolved.source(),
            QueryServiceConfig::default(),
        ));
    }

    let config_path = discover_config(start_dir.as_ref()).map_err(McpServerError::config)?;
    let Some(config_path) = config_path else {
        let error = resolve_database_path(LoadOptions {
            discovery_start_dir: Some(start_dir.as_ref().to_path_buf()),
            ..LoadOptions::default()
        })
        .err()
        .unwrap_or_else(|| semantic_graph_config::ConfigError::missing_database_path(None));
        return Err(McpServerError::config(error));
    };

    let config = load_config(&config_path).map_err(McpServerError::config)?;
    let resolved = resolve_database_path(LoadOptions {
        explicit_config_path: Some(config_path),
        ..LoadOptions::default()
    })
    .map_err(McpServerError::config)?;

    Ok(ResolvedServerConfig::new(
        resolved.path().to_path_buf(),
        resolved.source(),
        config.query_service().clone(),
    ))
}

#[cfg(test)]
mod tests {
    use crate::args::ServerArgs;
    use crate::args::server_args::resolve_server_config_from_start_dir;

    use clap::Parser;
    use semantic_graph_config::{ConfigError, ResolvedDatabasePathSource};
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn parses_default_stdio_command() -> Result<(), Box<dyn std::error::Error>> {
        let args = ServerArgs::try_parse_from(["semantic-graph-mcp-server"])?;

        assert_eq!(None, args.database_path());
        Ok(())
    }

    #[test]
    fn parses_database_path_override() -> Result<(), Box<dyn std::error::Error>> {
        let args = ServerArgs::try_parse_from([
            "semantic-graph-mcp-server",
            "--database-path",
            ".local/test.db",
        ])?;

        assert_eq!(Some(&PathBuf::from(".local/test.db")), args.database_path());
        Ok(())
    }

    #[test]
    fn rejects_unknown_transport_argument() {
        let result =
            ServerArgs::try_parse_from(["semantic-graph-mcp-server", "--transport", "http"]);

        assert!(result.is_err());
    }

    #[test]
    fn resolves_database_path_from_discovered_config() -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_dir("mcp-config-resolution")?;
        let nested = root.join("crates/example");
        fs::create_dir_all(&nested)?;
        write_config(&root, "content.db")?;

        let resolved = resolve_server_config_from_start_dir(
            &ServerArgs::try_parse_from(["semantic-graph-mcp-server"])?,
            &nested,
        )?;

        assert_eq!(
            root.join(".refactor-radar/content.db"),
            *resolved.database_path()
        );
        assert_eq!(
            ResolvedDatabasePathSource::ExplicitConfig,
            resolved.database_path_source()
        );
        Ok(())
    }

    #[test]
    fn database_path_override_bypasses_config() -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_dir("mcp-override-resolution")?;
        write_config(&root, "content.db")?;
        let override_path = root.join("override.db");
        let override_text = override_path.display().to_string();
        let args = ServerArgs::try_parse_from([
            "semantic-graph-mcp-server",
            "--database-path",
            override_text.as_str(),
        ])?;

        let resolved = resolve_server_config_from_start_dir(&args, &root)?;

        assert_eq!(override_path, *resolved.database_path());
        assert_eq!(
            ResolvedDatabasePathSource::ExplicitDatabasePath,
            resolved.database_path_source()
        );
        Ok(())
    }

    #[test]
    fn missing_config_returns_setup_error() -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_dir("mcp-missing-config")?;
        let args = ServerArgs::try_parse_from(["semantic-graph-mcp-server"])?;

        let error = resolve_server_config_from_start_dir(&args, &root)
            .err()
            .ok_or("expected missing config error")?;

        assert!(matches!(
            error,
            crate::McpServerError::Config {
                source: ConfigError::MissingDatabasePath { .. },
                ..
            }
        ));
        Ok(())
    }

    fn write_config(
        root: &std::path::Path,
        database_path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let config_dir = root.join(".refactor-radar");
        fs::create_dir_all(&config_dir)?;
        fs::write(
            config_dir.join("config.toml"),
            format!(
                r#"[database]
path = "{database_path}"

[query-service]
latest_run_limit = 7
max_search_limit = 70
"#
            ),
        )?;
        Ok(())
    }

    fn temp_dir(name: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let path = std::env::temp_dir().join(format!(
            "semantic-graph-mcp-server-{name}-{nanos}-{counter}"
        ));
        fs::create_dir_all(&path)?;
        Ok(path)
    }
}
