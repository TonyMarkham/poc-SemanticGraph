use std::path::PathBuf;

use semantic_graph_store::GraphStoreError;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, ExtractError>;

#[derive(Debug, Error)]
pub enum ExtractError {
    #[error("storage error: {0}")]
    Storage(#[from] GraphStoreError),

    #[error("io error during {context} path={path:?}: {source}")]
    Io {
        context: String,
        path: Option<PathBuf>,
        #[source]
        source: std::io::Error,
    },

    #[error("json error during {context}: {source}")]
    Json {
        context: String,
        #[source]
        source: serde_json::Error,
    },

    #[error(
        "json-rpc protocol error provider={provider} method={method} request_id={request_id:?}: {message}"
    )]
    JsonRpcProtocol {
        provider: String,
        method: String,
        request_id: Option<i64>,
        message: String,
    },

    #[error("provider response shape error provider={provider} method={method}: {message}")]
    ResponseShape {
        provider: String,
        method: String,
        message: String,
    },

    #[error("provider process error provider={provider} process={process}: {message}")]
    Process {
        provider: String,
        process: String,
        message: String,
    },

    #[error(
        "timeout provider={provider} method={method} request_id={request_id:?} after {timeout_ms}ms"
    )]
    Timeout {
        provider: String,
        method: String,
        request_id: Option<i64>,
        timeout_ms: u64,
    },

    #[error("invalid input path path={path:?} workspace_root={workspace_root:?}: {message}")]
    InvalidPath {
        path: PathBuf,
        workspace_root: PathBuf,
        message: String,
    },
}

impl ExtractError {
    pub fn io(context: impl Into<String>, path: Option<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            context: context.into(),
            path,
            source,
        }
    }

    pub fn json(context: impl Into<String>, source: serde_json::Error) -> Self {
        Self::Json {
            context: context.into(),
            source,
        }
    }

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
        }
    }

    pub fn response_shape(
        provider: impl Into<String>,
        method: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::ResponseShape {
            provider: provider.into(),
            method: method.into(),
            message: message.into(),
        }
    }

    pub fn process(
        provider: impl Into<String>,
        process: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::Process {
            provider: provider.into(),
            process: process.into(),
            message: message.into(),
        }
    }

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
        }
    }
}
