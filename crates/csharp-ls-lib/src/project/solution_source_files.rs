use crate::model::CSharpSolutionModel;

use std::path::PathBuf;

pub fn solution_source_files(model: &CSharpSolutionModel) -> Vec<PathBuf> {
    let mut files = model
        .projects
        .iter()
        .flat_map(|project| project.source_files.iter().cloned())
        .collect::<Vec<_>>();
    files.sort();
    files.dedup();
    files
}
