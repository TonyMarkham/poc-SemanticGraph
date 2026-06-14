use error_location::ErrorLocation;
use std::panic::Location;
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

    pub fn message(&self) -> &'static str {
        match self {
            Self::Database { .. } => "database error",
            Self::Migration { .. } => "migration error",
        }
    }

    pub fn location(&self) -> ErrorLocation {
        match self {
            Self::Database { location, .. } | Self::Migration { location, .. } => *location,
        }
    }
}
