use error_location::ErrorLocation;
use std::{panic::Location, path::PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum McpServerError {
    #[error("configuration error at {location}")]
    Config {
        #[source]
        source: semantic_graph_config::ConfigError,
        location: ErrorLocation,
    },

    #[error("io error during {operation} at {location}")]
    Io {
        operation: &'static str,
        path: Option<PathBuf>,
        #[source]
        source: std::io::Error,
        location: ErrorLocation,
    },

    #[error("mcp service error at {location}")]
    Rmcp {
        #[source]
        source: Box<rmcp::RmcpError>,
        location: ErrorLocation,
    },
}

impl McpServerError {
    #[track_caller]
    pub fn config(source: semantic_graph_config::ConfigError) -> Self {
        Self::Config {
            source,
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn io(operation: &'static str, path: Option<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            operation,
            path,
            source,
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn rmcp(source: rmcp::RmcpError) -> Self {
        Self::Rmcp {
            source: Box::new(source),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    pub fn user_message(&self) -> String {
        match self {
            Self::Config { source, .. } => format!(
                "{}. Configure .refactor-radar/config.toml [database].path or pass --database-path.",
                source.message()
            ),
            Self::Io {
                operation, path, ..
            } => match path {
                Some(path) => format!("{operation} failed for {}", path.display()),
                None => format!("{operation} failed"),
            },
            Self::Rmcp { source, .. } => format!("MCP service failed: {source}"),
        }
    }
}
