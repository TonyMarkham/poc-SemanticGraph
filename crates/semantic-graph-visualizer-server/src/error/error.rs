use error_location::ErrorLocation;
use std::panic::Location;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VisualizerServerError {
    #[error("database error at {location}")]
    Database {
        #[source]
        source: sqlx::Error,
        location: ErrorLocation,
    },

    #[error("io error at {location}")]
    Io {
        #[source]
        source: std::io::Error,
        location: ErrorLocation,
    },

    #[error("invalid configuration at {location}: {message}")]
    InvalidConfig {
        message: String,
        location: ErrorLocation,
    },

    #[error("invalid params at {location}: {message}")]
    InvalidParams {
        message: String,
        location: ErrorLocation,
    },

    #[error("invalid request at {location}: {message}")]
    InvalidRequest {
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

impl VisualizerServerError {
    #[track_caller]
    pub fn database(source: sqlx::Error) -> Self {
        Self::Database {
            source,
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn io(source: std::io::Error) -> Self {
        Self::Io {
            source,
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn invalid_config(message: String) -> Self {
        Self::InvalidConfig {
            message,
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn invalid_params(message: String) -> Self {
        Self::InvalidParams {
            message,
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn invalid_request(message: String) -> Self {
        Self::InvalidRequest {
            message,
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn not_found(message: String) -> Self {
        Self::NotFound {
            message,
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
            Self::Io { .. } => "io error",
            Self::InvalidConfig { message, .. }
            | Self::InvalidParams { message, .. }
            | Self::InvalidRequest { message, .. }
            | Self::NotFound { message, .. } => message,
            Self::Json { .. } => "json error",
        }
    }
}
