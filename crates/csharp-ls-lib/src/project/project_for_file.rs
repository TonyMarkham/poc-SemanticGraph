use crate::{
    CSharpLsLibError, CSharpLsLibResult,
    model::{CSharpProjectMatch, CSharpSolutionModel},
};

use std::path::Path;

pub fn project_for_file(
    model: &CSharpSolutionModel,
    file_path: &Path,
) -> CSharpLsLibResult<CSharpProjectMatch> {
    let file_path = file_path
        .canonicalize()
        .map_err(|source| CSharpLsLibError::io("canonicalize C# source file", None, source))?;
    let matches = model
        .projects
        .iter()
        .filter(|project| {
            project
                .source_files
                .iter()
                .any(|source| source == &file_path)
        })
        .collect::<Vec<_>>();

    match matches.as_slice() {
        [project] => Ok(CSharpProjectMatch {
            project_path: project.project_path.clone(),
            file_path,
        }),
        [] => Err(CSharpLsLibError::invalid_path(
            file_path,
            "C# file is not part of the resolved solution",
        )),
        _ => Err(CSharpLsLibError::response_shape(
            "project_for_file",
            format!(
                "C# file {} is included by multiple projects; project disambiguation is not supported in the first slice",
                file_path.display()
            ),
        )),
    }
}
