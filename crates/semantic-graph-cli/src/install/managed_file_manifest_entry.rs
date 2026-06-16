use crate::install::FileActionKind;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ManagedFileManifestEntry {
    pub path: String,
    pub sha256: String,
    pub action: FileActionKind,
}

impl ManagedFileManifestEntry {
    pub fn new(path: impl Into<String>, sha256: impl Into<String>, action: FileActionKind) -> Self {
        Self {
            path: path.into(),
            sha256: sha256.into(),
            action,
        }
    }
}
