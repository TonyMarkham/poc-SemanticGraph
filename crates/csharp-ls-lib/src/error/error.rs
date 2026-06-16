use error_location::ErrorLocation;
use std::{io, panic::Location, path::PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CSharpLsLibError {
    #[error("csharp-ls setup error binary={binary:?} at {location}: {message}")]
    Setup {
        binary: PathBuf,
        message: String,
        location: ErrorLocation,
    },

    #[error("io error during {context} path={path:?} at {location}")]
    Io {
        context: &'static str,
        path: Option<PathBuf>,
        #[source]
        source: io::Error,
        location: ErrorLocation,
    },

    #[error("json error during {context} at {location}")]
    Json {
        context: &'static str,
        #[source]
        source: serde_json::Error,
        location: ErrorLocation,
    },

    #[error(
        "json-rpc protocol error method={method} request_id={request_id:?} at {location}: {message}"
    )]
    Protocol {
        method: String,
        request_id: Option<i64>,
        message: String,
        location: ErrorLocation,
    },

    #[error("timeout method={method} request_id={request_id:?} after {timeout_ms}ms at {location}")]
    Timeout {
        method: String,
        request_id: Option<i64>,
        timeout_ms: u64,
        location: ErrorLocation,
    },

    #[error("invalid csharp path path={path:?} at {location}: {message}")]
    InvalidPath {
        path: PathBuf,
        message: String,
        location: ErrorLocation,
    },

    #[error("csharp-ls response shape error method={method} at {location}: {message}")]
    ResponseShape {
        method: String,
        message: String,
        location: ErrorLocation,
    },
}

impl CSharpLsLibError {
    #[track_caller]
    pub(crate) fn setup(binary: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        Self::Setup {
            binary: binary.into(),
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub(crate) fn missing_binary(binary: impl Into<PathBuf>) -> Self {
        Self::setup(
            binary,
            "missing csharp-ls binary; run `dotnet tool install --global csharp-ls` and verify with `csharp-ls --help`",
        )
    }

    #[track_caller]
    pub(crate) fn io(context: &'static str, path: Option<PathBuf>, source: io::Error) -> Self {
        Self::Io {
            context,
            path,
            source,
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub(crate) fn json(context: &'static str, source: serde_json::Error) -> Self {
        Self::Json {
            context,
            source,
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub(crate) fn protocol(
        method: impl Into<String>,
        request_id: Option<i64>,
        message: impl Into<String>,
    ) -> Self {
        Self::Protocol {
            method: method.into(),
            request_id,
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub(crate) fn timeout(
        method: impl Into<String>,
        request_id: Option<i64>,
        timeout_ms: u64,
    ) -> Self {
        Self::Timeout {
            method: method.into(),
            request_id,
            timeout_ms,
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub(crate) fn invalid_path(path: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        Self::InvalidPath {
            path: path.into(),
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub(crate) fn response_shape(method: impl Into<String>, message: impl Into<String>) -> Self {
        Self::ResponseShape {
            method: method.into(),
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    pub fn message(&self) -> &'static str {
        match self {
            Self::Setup { .. } => "csharp-ls setup error",
            Self::Io { .. } => "io error",
            Self::Json { .. } => "json error",
            Self::Protocol { .. } => "json-rpc protocol error",
            Self::Timeout { .. } => "timeout",
            Self::InvalidPath { .. } => "invalid csharp path",
            Self::ResponseShape { .. } => "csharp-ls response shape error",
        }
    }

    pub fn location(&self) -> ErrorLocation {
        match self {
            Self::Setup { location, .. }
            | Self::Io { location, .. }
            | Self::Json { location, .. }
            | Self::Protocol { location, .. }
            | Self::Timeout { location, .. }
            | Self::InvalidPath { location, .. }
            | Self::ResponseShape { location, .. } => *location,
        }
    }
}
