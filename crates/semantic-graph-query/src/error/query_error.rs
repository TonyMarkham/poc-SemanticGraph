use error_location::ErrorLocation;
use std::panic::Location;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum QueryError {
    #[error("database error at {location}")]
    Database {
        #[source]
        source: sqlx::Error,
        location: ErrorLocation,
    },

    #[error("invalid params at {location}: {message}")]
    InvalidParams {
        message: String,
        location: ErrorLocation,
    },

    #[error("not found at {location}: {message}")]
    NotFound {
        message: String,
        location: ErrorLocation,
    },

    #[error("json error at {location}")]
    Json {
        #[source]
        source: serde_json::Error,
        location: ErrorLocation,
    },
}

impl QueryError {
    #[track_caller]
    pub fn database(source: sqlx::Error) -> Self {
        Self::Database {
            source,
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self::InvalidParams {
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound {
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn json(source: serde_json::Error) -> Self {
        Self::Json {
            source,
            location: ErrorLocation::from(Location::caller()),
        }
    }

    pub fn message(&self) -> &str {
        match self {
            Self::Database { .. } => "database error",
            Self::InvalidParams { message, .. } | Self::NotFound { message, .. } => message,
            Self::Json { .. } => "json error",
        }
    }
}
