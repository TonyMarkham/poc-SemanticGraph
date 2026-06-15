use error_location::ErrorLocation;
use semantic_graph_config::ConfigError;
use std::{io, panic::Location, path::PathBuf};
use thiserror::Error;
use tokio::{
    sync::{mpsc, oneshot},
    task::JoinError,
};

#[derive(Debug, Error)]
pub enum DbManagerError {
    #[error("database error at {location}")]
    Database {
        #[source]
        source: sqlx::Error,
        location: ErrorLocation,
    },

    #[error("migration error at {location}")]
    Migration {
        #[source]
        source: sqlx::migrate::MigrateError,
        location: ErrorLocation,
    },

    #[error("io error during {context} path={path:?} at {location}")]
    Io {
        context: String,
        path: Option<PathBuf>,
        #[source]
        source: io::Error,
        location: ErrorLocation,
    },

    #[error("configuration error at {location}")]
    Config {
        #[source]
        source: Box<ConfigError>,
        location: ErrorLocation,
    },

    #[error("db write manager closed during {context} at {location}")]
    Closed {
        context: String,
        location: ErrorLocation,
    },

    #[error("db write manager task failed at {location}")]
    WorkerTask {
        #[source]
        source: JoinError,
        location: ErrorLocation,
    },
}

impl DbManagerError {
    #[track_caller]
    pub fn database(source: sqlx::Error) -> Self {
        Self::Database {
            source,
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn migration(source: sqlx::migrate::MigrateError) -> Self {
        Self::Migration {
            source,
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn io(context: impl Into<String>, path: Option<PathBuf>, source: io::Error) -> Self {
        Self::Io {
            context: context.into(),
            path,
            source,
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn config(source: ConfigError) -> Self {
        Self::Config {
            source: Box::new(source),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn closed(context: impl Into<String>) -> Self {
        Self::Closed {
            context: context.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn worker_task(source: JoinError) -> Self {
        Self::WorkerTask {
            source,
            location: ErrorLocation::from(Location::caller()),
        }
    }

    pub fn message(&self) -> &'static str {
        match self {
            Self::Database { .. } => "database error",
            Self::Migration { .. } => "migration error",
            Self::Io { .. } => "io error",
            Self::Config { .. } => "configuration error",
            Self::Closed { .. } => "db write manager closed",
            Self::WorkerTask { .. } => "db write manager task failed",
        }
    }

    pub fn location(&self) -> ErrorLocation {
        match self {
            Self::Database { location, .. }
            | Self::Migration { location, .. }
            | Self::Io { location, .. }
            | Self::Config { location, .. }
            | Self::Closed { location, .. }
            | Self::WorkerTask { location, .. } => *location,
        }
    }
}

impl<T> From<mpsc::error::SendError<T>> for DbManagerError {
    #[track_caller]
    fn from(_source: mpsc::error::SendError<T>) -> Self {
        Self::closed("send write command")
    }
}

impl From<oneshot::error::RecvError> for DbManagerError {
    #[track_caller]
    fn from(_source: oneshot::error::RecvError) -> Self {
        Self::closed("receive write command response")
    }
}
