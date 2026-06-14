use crate::model::RustTargetKind;

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustTarget {
    pub name: String,
    pub kind: RustTargetKind,
    pub root_file: PathBuf,
}
