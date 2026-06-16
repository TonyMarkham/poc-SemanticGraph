use crate::model::CSharpProjectModel;

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CSharpSolutionModel {
    pub solution_path: PathBuf,
    pub root_dir: PathBuf,
    pub projects: Vec<CSharpProjectModel>,
}
