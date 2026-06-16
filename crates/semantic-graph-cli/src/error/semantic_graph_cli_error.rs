use crate::install::{CodexInstallReport, CodexUninstallReport};
use error_location::ErrorLocation;
use std::{panic::Location, path::PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SemanticGraphCliError {
    #[error("agent asset error at {location}")]
    AgentAssets {
        #[source]
        source: semantic_graph_agent_assets::AgentAssetsError,
        location: ErrorLocation,
    },

    #[error("config TOML parse error at {location}")]
    ConfigTomlParse {
        path: PathBuf,
        #[source]
        source: Box<toml::de::Error>,
        location: ErrorLocation,
    },

    #[error("config TOML serialization failed at {location}")]
    ConfigTomlSerialize {
        #[source]
        source: Box<toml::ser::Error>,
        location: ErrorLocation,
    },

    #[error("install manifest serialization failed at {location}")]
    ManifestSerialize {
        #[source]
        source: Box<serde_json::Error>,
        location: ErrorLocation,
    },

    #[error("install manifest parse failed at {location}")]
    ManifestParse {
        path: PathBuf,
        #[source]
        source: Box<serde_json::Error>,
        location: ErrorLocation,
    },

    #[error("missing install manifest at {location}: {path}")]
    MissingManifest {
        path: PathBuf,
        location: ErrorLocation,
    },

    #[error("invalid install manifest at {location}: {path}")]
    InvalidManifest {
        path: PathBuf,
        message: String,
        location: ErrorLocation,
    },

    #[error("io error during {operation} at {location}")]
    Io {
        operation: &'static str,
        path: Option<PathBuf>,
        #[source]
        source: Box<std::io::Error>,
        location: ErrorLocation,
    },

    #[error("invalid project at {location}: {message}")]
    InvalidProject {
        message: String,
        location: ErrorLocation,
    },

    #[error("invalid install path at {location}: {path}")]
    InvalidInstallPath {
        path: PathBuf,
        message: String,
        location: ErrorLocation,
    },

    #[error("install refused writes at {location}")]
    RefusedWrites {
        report: Box<CodexInstallReport>,
        location: ErrorLocation,
    },

    #[error("uninstall refused changes at {location}")]
    RefusedUninstall {
        report: Box<CodexUninstallReport>,
        location: ErrorLocation,
    },
}

impl SemanticGraphCliError {
    #[track_caller]
    pub fn agent_assets(source: semantic_graph_agent_assets::AgentAssetsError) -> Self {
        Self::AgentAssets {
            source,
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn config_toml_parse(path: PathBuf, source: toml::de::Error) -> Self {
        Self::ConfigTomlParse {
            path,
            source: Box::new(source),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn config_toml_serialize(source: toml::ser::Error) -> Self {
        Self::ConfigTomlSerialize {
            source: Box::new(source),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn manifest_serialize(source: serde_json::Error) -> Self {
        Self::ManifestSerialize {
            source: Box::new(source),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn manifest_parse(path: PathBuf, source: serde_json::Error) -> Self {
        Self::ManifestParse {
            path,
            source: Box::new(source),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn missing_manifest(path: PathBuf) -> Self {
        Self::MissingManifest {
            path,
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn invalid_manifest(path: PathBuf, message: impl Into<String>) -> Self {
        Self::InvalidManifest {
            path,
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn io(operation: &'static str, path: Option<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            operation,
            path,
            source: Box::new(source),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn invalid_project(message: impl Into<String>) -> Self {
        Self::InvalidProject {
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn invalid_install_path(path: PathBuf, message: impl Into<String>) -> Self {
        Self::InvalidInstallPath {
            path,
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn refused_writes(report: CodexInstallReport) -> Self {
        Self::RefusedWrites {
            report: Box::new(report),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn refused_uninstall(report: CodexUninstallReport) -> Self {
        Self::RefusedUninstall {
            report: Box::new(report),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    pub fn exit_code(&self) -> i32 {
        match self {
            Self::AgentAssets { .. } => 2,
            Self::ConfigTomlParse { .. }
            | Self::ConfigTomlSerialize { .. }
            | Self::ManifestParse { .. }
            | Self::ManifestSerialize { .. }
            | Self::MissingManifest { .. }
            | Self::InvalidManifest { .. } => 3,
            Self::Io { .. } => 1,
            Self::InvalidProject { .. } | Self::InvalidInstallPath { .. } => 4,
            Self::RefusedWrites { .. } | Self::RefusedUninstall { .. } => 5,
        }
    }

    pub fn user_message(&self) -> String {
        match self {
            Self::AgentAssets { source, .. } => source.user_message(),
            Self::ConfigTomlParse { path, source, .. } => {
                format!("failed to parse {}: {source}", path.display())
            }
            Self::ConfigTomlSerialize { .. } => "failed to serialize Codex config TOML".to_string(),
            Self::ManifestParse { path, source, .. } => {
                format!("failed to parse {}: {source}", path.display())
            }
            Self::MissingManifest { path, .. } => {
                format!("missing SemanticGraph install manifest: {}", path.display())
            }
            Self::InvalidManifest { path, message, .. } => {
                format!(
                    "invalid SemanticGraph install manifest {}: {message}",
                    path.display()
                )
            }
            Self::ManifestSerialize { .. } => {
                "failed to serialize SemanticGraph install manifest".to_string()
            }
            Self::Io {
                operation, path, ..
            } => match path {
                Some(path) => format!("{operation} failed for {}", path.display()),
                None => format!("{operation} failed"),
            },
            Self::InvalidProject { message, .. } => format!("invalid project: {message}"),
            Self::InvalidInstallPath { path, message, .. } => {
                format!("invalid install path {}: {message}", path.display())
            }
            Self::RefusedWrites { report, .. } => report.lines().join("\n"),
            Self::RefusedUninstall { report, .. } => report.lines().join("\n"),
        }
    }
}
