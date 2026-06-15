use crate::{VisualizerServerError, VisualizerServerResult, config::server_args::ServerArgs};

use clap::Parser;
use semantic_graph_config::{LoadOptions, resolve_database_path};
use std::{
    env,
    net::SocketAddr,
    path::{Path, PathBuf},
};

const DEFAULT_DATABASE_PATH: &str = ".local/rust-workspace-extract.db";
const DEFAULT_BIND_ADDRESS: &str = "127.0.0.1:5179";
const DATABASE_PATH_ENV: &str = "SEMANTIC_GRAPH_DB_PATH";
const BIND_ADDRESS_ENV: &str = "SEMANTIC_GRAPH_VISUALIZER_BIND";

#[derive(Debug, Clone)]
pub struct ServerConfig {
    database_path: PathBuf,
    bind: SocketAddr,
}

impl ServerConfig {
    pub fn from_env_and_args() -> VisualizerServerResult<Self> {
        let args = ServerArgs::parse();
        let database_path = resolve_database_path(LoadOptions {
            explicit_database_path: args
                .database_path
                .or_else(|| env::var_os(DATABASE_PATH_ENV).map(PathBuf::from)),
            explicit_config_path: args.config,
            discovery_start_dir: None,
            default_database_path: Some(PathBuf::from(DEFAULT_DATABASE_PATH)),
        })
        .map_err(VisualizerServerError::config)?
        .into_path();

        let bind = match args.bind {
            Some(value) => value,
            None => env::var(BIND_ADDRESS_ENV)
                .map(|value| parse_socket_addr(&value))
                .unwrap_or_else(|_| parse_socket_addr(DEFAULT_BIND_ADDRESS))?,
        };

        Ok(Self {
            database_path,
            bind,
        })
    }

    pub fn new(database_path: PathBuf, bind: SocketAddr) -> Self {
        Self {
            database_path,
            bind,
        }
    }

    pub fn database_path(&self) -> &Path {
        &self.database_path
    }

    pub fn bind(&self) -> SocketAddr {
        self.bind
    }
}

fn parse_socket_addr(value: &str) -> VisualizerServerResult<SocketAddr> {
    value.parse().map_err(|source| {
        VisualizerServerError::invalid_config(format!("invalid bind address '{value}': {source}"))
    })
}
