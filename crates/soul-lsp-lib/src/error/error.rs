use error_location::ErrorLocation;
use std::{error::Error, io, panic::Location, path::PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SoulLspLibError {
    #[error("soul project error during {context} at {location}")]
    Project {
        context: &'static str,
        #[source]
        source: Box<dyn Error + Send + Sync + 'static>,
        location: ErrorLocation,
    },

    #[error("soul analysis error during {context} at {location}: {message}")]
    AnalysisMessage {
        context: &'static str,
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

    #[error("invalid soul path path={path:?} at {location}: {message}")]
    InvalidPath {
        path: PathBuf,
        message: String,
        location: ErrorLocation,
    },
}

impl SoulLspLibError {
    #[track_caller]
    pub(crate) fn project(
        context: &'static str,
        source: impl Error + Send + Sync + 'static,
    ) -> Self {
        Self::Project {
            context,
            source: Box::new(source),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub(crate) fn analysis_message(context: &'static str, message: impl Into<String>) -> Self {
        Self::AnalysisMessage {
            context,
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
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
    pub(crate) fn invalid_path(path: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        Self::InvalidPath {
            path: path.into(),
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    pub fn message(&self) -> &'static str {
        match self {
            Self::Project { .. } => "soul project error",
            Self::AnalysisMessage { .. } => "soul analysis error",
            Self::Io { .. } => "io error",
            Self::InvalidPath { .. } => "invalid soul path",
        }
    }

    pub fn location(&self) -> ErrorLocation {
        match self {
            Self::Project { location, .. }
            | Self::AnalysisMessage { location, .. }
            | Self::Io { location, .. }
            | Self::InvalidPath { location, .. } => *location,
        }
    }
}
