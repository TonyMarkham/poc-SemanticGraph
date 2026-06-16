use error_location::ErrorLocation;
use semantic_graph_config::ConfigError;
use semantic_graph_db_manager::DbManagerError;
use std::{io, panic::Location, path::PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExtractError {
    #[error("storage error at {location}")]
    Storage {
        #[source]
        source: Box<DbManagerError>,
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

    #[error("json error during {context} at {location}")]
    Json {
        context: String,
        #[source]
        source: serde_json::Error,
        location: ErrorLocation,
    },

    #[error(
        "json-rpc protocol error provider={provider} method={method} request_id={request_id:?} at {location}: {message}"
    )]
    JsonRpcProtocol {
        provider: String,
        method: String,
        request_id: Option<i64>,
        message: String,
        location: ErrorLocation,
    },

    #[error(
        "provider response shape error provider={provider} method={method} at {location}: {message}"
    )]
    ResponseShape {
        provider: String,
        method: String,
        message: String,
        location: ErrorLocation,
    },

    #[error(
        "provider process error provider={provider} process={process} at {location}: {message}"
    )]
    Process {
        provider: String,
        process: String,
        message: String,
        location: ErrorLocation,
    },

    #[error(
        "timeout provider={provider} method={method} request_id={request_id:?} after {timeout_ms}ms at {location}"
    )]
    Timeout {
        provider: String,
        method: String,
        request_id: Option<i64>,
        timeout_ms: u64,
        location: ErrorLocation,
    },

    #[error(
        "invalid input path path={path:?} workspace_root={workspace_root:?} at {location}: {message}"
    )]
    InvalidPath {
        path: PathBuf,
        workspace_root: PathBuf,
        message: String,
        location: ErrorLocation,
    },

    #[error("rust-analyzer-lib error during {context} at {location}")]
    RustAnalyzerLib {
        context: String,
        #[source]
        source: Box<rust_analyzer_lib::RustAnalyzerLibError>,
        location: ErrorLocation,
    },

    #[error("csharp-ls-lib error during {context} at {location}")]
    CSharpLsLib {
        context: String,
        #[source]
        source: Box<csharp_ls_lib::CSharpLsLibError>,
        location: ErrorLocation,
    },

    #[error("configuration error at {location}")]
    Config {
        #[source]
        source: Box<ConfigError>,
        location: ErrorLocation,
    },
}

impl ExtractError {
    #[track_caller]
    pub fn storage(source: DbManagerError) -> Self {
        Self::Storage {
            source: Box::new(source),
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
    pub fn json(context: impl Into<String>, source: serde_json::Error) -> Self {
        Self::Json {
            context: context.into(),
            source,
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn protocol(
        provider: impl Into<String>,
        method: impl Into<String>,
        request_id: Option<i64>,
        message: impl Into<String>,
    ) -> Self {
        Self::JsonRpcProtocol {
            provider: provider.into(),
            method: method.into(),
            request_id,
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn response_shape(
        provider: impl Into<String>,
        method: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::ResponseShape {
            provider: provider.into(),
            method: method.into(),
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn process(
        provider: impl Into<String>,
        process: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::Process {
            provider: provider.into(),
            process: process.into(),
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn timeout(
        provider: impl Into<String>,
        method: impl Into<String>,
        request_id: Option<i64>,
        timeout_ms: u64,
    ) -> Self {
        Self::Timeout {
            provider: provider.into(),
            method: method.into(),
            request_id,
            timeout_ms,
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn invalid_path(
        path: impl Into<PathBuf>,
        workspace_root: impl Into<PathBuf>,
        message: impl Into<String>,
    ) -> Self {
        Self::InvalidPath {
            path: path.into(),
            workspace_root: workspace_root.into(),
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn rust_analyzer_lib(
        context: impl Into<String>,
        source: rust_analyzer_lib::RustAnalyzerLibError,
    ) -> Self {
        Self::RustAnalyzerLib {
            context: context.into(),
            source: Box::new(source),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn csharp_ls_lib(
        context: impl Into<String>,
        source: csharp_ls_lib::CSharpLsLibError,
    ) -> Self {
        Self::CSharpLsLib {
            context: context.into(),
            source: Box::new(source),
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

    pub fn message(&self) -> &'static str {
        match self {
            Self::Storage { .. } => "storage error",
            Self::Io { .. } => "io error",
            Self::Json { .. } => "json error",
            Self::JsonRpcProtocol { .. } => "json-rpc protocol error",
            Self::ResponseShape { .. } => "provider response shape error",
            Self::Process { .. } => "provider process error",
            Self::Timeout { .. } => "timeout",
            Self::InvalidPath { .. } => "invalid input path",
            Self::RustAnalyzerLib { .. } => "rust-analyzer-lib error",
            Self::CSharpLsLib { .. } => "csharp-ls-lib error",
            Self::Config { .. } => "configuration error",
        }
    }

    pub fn location(&self) -> ErrorLocation {
        match self {
            Self::Storage { location, .. }
            | Self::Io { location, .. }
            | Self::Json { location, .. }
            | Self::JsonRpcProtocol { location, .. }
            | Self::ResponseShape { location, .. }
            | Self::Process { location, .. }
            | Self::Timeout { location, .. }
            | Self::InvalidPath { location, .. }
            | Self::RustAnalyzerLib { location, .. }
            | Self::CSharpLsLib { location, .. }
            | Self::Config { location, .. } => *location,
        }
    }
}
