use error_location::ErrorLocation;
use std::{panic::Location, path::PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AgentAssetsError {
    #[error("io error during {operation} at {location}")]
    Io {
        operation: &'static str,
        path: Option<PathBuf>,
        #[source]
        source: Box<std::io::Error>,
        location: ErrorLocation,
    },

    #[error("invalid manifest TOML at {location}")]
    ManifestToml {
        path: PathBuf,
        #[source]
        source: Box<toml::de::Error>,
        location: ErrorLocation,
    },

    #[error("invalid manifest at {location}: {message}")]
    InvalidManifest {
        message: String,
        location: ErrorLocation,
    },

    #[error("missing manifest-declared fragment at {location}: {path}")]
    MissingFragment {
        path: PathBuf,
        location: ErrorLocation,
    },

    #[error("duplicate output path at {location}: {output_path}")]
    DuplicateOutputPath {
        output_path: PathBuf,
        location: ErrorLocation,
    },

    #[error("output path escapes expected root at {location}: {output_path}")]
    OutputPathEscapesExpectedRoot {
        output_path: PathBuf,
        expected_root: PathBuf,
        location: ErrorLocation,
    },

    #[error("TOML serialization failed for {artifact} at {location}")]
    TomlSerialize {
        artifact: String,
        #[source]
        source: Box<toml::ser::Error>,
        location: ErrorLocation,
    },

    #[error("generated asset drift at {location}")]
    Drift {
        report: String,
        location: ErrorLocation,
    },
}

impl AgentAssetsError {
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
    pub fn manifest_toml(path: PathBuf, source: toml::de::Error) -> Self {
        Self::ManifestToml {
            path,
            source: Box::new(source),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn invalid_manifest(message: impl Into<String>) -> Self {
        Self::InvalidManifest {
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn missing_fragment(path: PathBuf) -> Self {
        Self::MissingFragment {
            path,
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn duplicate_output_path(output_path: PathBuf) -> Self {
        Self::DuplicateOutputPath {
            output_path,
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn output_path_escapes_expected_root(output_path: PathBuf, expected_root: PathBuf) -> Self {
        Self::OutputPathEscapesExpectedRoot {
            output_path,
            expected_root,
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn toml_serialize(artifact: impl Into<String>, source: toml::ser::Error) -> Self {
        Self::TomlSerialize {
            artifact: artifact.into(),
            source: Box::new(source),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn drift(report: impl Into<String>) -> Self {
        Self::Drift {
            report: report.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Io { .. } | Self::TomlSerialize { .. } => 1,
            Self::ManifestToml { .. } | Self::InvalidManifest { .. } => 2,
            Self::MissingFragment { .. } => 3,
            Self::DuplicateOutputPath { .. } => 4,
            Self::OutputPathEscapesExpectedRoot { .. } => 5,
            Self::Drift { .. } => 6,
        }
    }

    pub fn user_message(&self) -> String {
        match self {
            Self::Io {
                operation, path, ..
            } => match path {
                Some(path) => format!("{operation} failed for {}", path.display()),
                None => format!("{operation} failed"),
            },
            Self::ManifestToml { path, source, .. } => {
                format!("invalid manifest TOML in {}: {source}", path.display())
            }
            Self::InvalidManifest { message, .. } => format!("invalid manifest: {message}"),
            Self::MissingFragment { path, .. } => {
                format!("missing manifest-declared fragment: {}", path.display())
            }
            Self::DuplicateOutputPath { output_path, .. } => {
                format!("duplicate generated output path: {}", output_path.display())
            }
            Self::OutputPathEscapesExpectedRoot {
                output_path,
                expected_root,
                ..
            } => format!(
                "generated output path {} escapes expected root {}",
                output_path.display(),
                expected_root.display()
            ),
            Self::TomlSerialize { artifact, .. } => {
                format!("failed to serialize TOML for {artifact}")
            }
            Self::Drift { report, .. } => report.clone(),
        }
    }
}
