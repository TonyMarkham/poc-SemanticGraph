use error_location::ErrorLocation;
use semantic_graph_config::ConfigError;
use semantic_graph_db_manager::DbManagerError;
use std::{io, panic::Location, path::PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GraphStoreError {
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

    #[error("db manager error at {location}")]
    DbManager {
        #[source]
        source: Box<DbManagerError>,
        location: ErrorLocation,
    },
}

impl GraphStoreError {
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
    pub fn db_manager(source: DbManagerError) -> Self {
        Self::DbManager {
            source: Box::new(source),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    pub fn message(&self) -> &'static str {
        match self {
            Self::Database { .. } => "database error",
            Self::Migration { .. } => "migration error",
            Self::Io { .. } => "io error",
            Self::Config { .. } => "configuration error",
            Self::DbManager { .. } => "db manager error",
        }
    }

    pub fn location(&self) -> ErrorLocation {
        match self {
            Self::Database { location, .. }
            | Self::Migration { location, .. }
            | Self::Io { location, .. }
            | Self::Config { location, .. }
            | Self::DbManager { location, .. } => *location,
        }
    }
}
