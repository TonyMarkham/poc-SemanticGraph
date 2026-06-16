use error_location::ErrorLocation;
use std::{io, panic::Location, path::PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("io error during {context} path={path:?} at {location}")]
    Io {
        context: String,
        path: Option<PathBuf>,
        #[source]
        source: io::Error,
        location: ErrorLocation,
    },

    #[error("toml error config_path={config_path:?} at {location}")]
    Toml {
        config_path: PathBuf,
        #[source]
        source: Box<toml::de::Error>,
        location: ErrorLocation,
    },

    #[error("missing database path config_path={config_path:?} at {location}")]
    MissingDatabasePath {
        config_path: Option<PathBuf>,
        location: ErrorLocation,
    },

    #[error("invalid writer setting setting={setting} at {location}: {message}")]
    InvalidWriterSetting {
        setting: String,
        message: String,
        location: ErrorLocation,
    },

    #[error("invalid extractor setting setting={setting} at {location}: {message}")]
    InvalidExtractorSetting {
        setting: String,
        message: String,
        location: ErrorLocation,
    },

    #[error("invalid csharp setting setting={setting} at {location}: {message}")]
    InvalidCSharpSetting {
        setting: String,
        message: String,
        location: ErrorLocation,
    },
}

impl ConfigError {
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
    pub fn toml(config_path: impl Into<PathBuf>, source: toml::de::Error) -> Self {
        Self::Toml {
            config_path: config_path.into(),
            source: Box::new(source),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn missing_database_path(config_path: Option<PathBuf>) -> Self {
        Self::MissingDatabasePath {
            config_path,
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn invalid_writer_setting(setting: impl Into<String>, message: impl Into<String>) -> Self {
        Self::InvalidWriterSetting {
            setting: setting.into(),
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn invalid_extractor_setting(
        setting: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::InvalidExtractorSetting {
            setting: setting.into(),
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn invalid_csharp_setting(setting: impl Into<String>, message: impl Into<String>) -> Self {
        Self::InvalidCSharpSetting {
            setting: setting.into(),
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    pub fn message(&self) -> &'static str {
        match self {
            Self::Io { .. } => "io error",
            Self::Toml { .. } => "toml error",
            Self::MissingDatabasePath { .. } => "missing database path",
            Self::InvalidWriterSetting { .. } => "invalid writer setting",
            Self::InvalidExtractorSetting { .. } => "invalid extractor setting",
            Self::InvalidCSharpSetting { .. } => "invalid csharp setting",
        }
    }

    pub fn location(&self) -> ErrorLocation {
        match self {
            Self::Io { location, .. }
            | Self::Toml { location, .. }
            | Self::MissingDatabasePath { location, .. }
            | Self::InvalidWriterSetting { location, .. }
            | Self::InvalidExtractorSetting { location, .. }
            | Self::InvalidCSharpSetting { location, .. } => *location,
        }
    }
}
