use crate::{
    CSharpLsLibError, CSharpLsLibResult,
    model::{CSharpProjectModel, CSharpSolutionModel},
    project::project_file_source_files,
};

use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn load_solution(solution_path: &Path) -> CSharpLsLibResult<CSharpSolutionModel> {
    let solution_path = solution_path
        .canonicalize()
        .map_err(|source| CSharpLsLibError::io("canonicalize C# solution path", None, source))?;
    if !is_solution_file(&solution_path) {
        return Err(CSharpLsLibError::invalid_path(
            solution_path,
            "solution path must end in .slnx or .sln",
        ));
    }

    let root_dir = solution_path
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| {
            CSharpLsLibError::invalid_path(&solution_path, "solution path has no parent directory")
        })?;
    let contents = fs::read_to_string(&solution_path).map_err(|source| {
        CSharpLsLibError::io("read C# solution file", Some(solution_path.clone()), source)
    })?;
    let project_paths = if extension_is(&solution_path, "slnx") {
        parse_slnx_projects(&root_dir, &contents)
    } else {
        parse_sln_projects(&root_dir, &contents)
    };
    if project_paths.is_empty() {
        return Err(CSharpLsLibError::response_shape(
            "load_solution",
            "solution contained no C# projects",
        ));
    }

    let mut projects = Vec::with_capacity(project_paths.len());
    for project_path in project_paths {
        projects.push(CSharpProjectModel {
            source_files: project_file_source_files(&project_path)?,
            project_path,
        });
    }

    Ok(CSharpSolutionModel {
        solution_path,
        root_dir,
        projects,
    })
}

fn parse_slnx_projects(root_dir: &Path, contents: &str) -> Vec<PathBuf> {
    let mut projects = Vec::new();
    for line in contents.lines() {
        let Some(path) = attribute_value(line, "Path") else {
            continue;
        };
        let project_path = root_dir.join(path);
        if extension_is(&project_path, "csproj") {
            projects.push(project_path);
        }
    }
    projects.sort();
    projects
}

fn parse_sln_projects(root_dir: &Path, contents: &str) -> Vec<PathBuf> {
    let mut projects = Vec::new();
    for line in contents.lines() {
        if !line.trim_start().starts_with("Project(") {
            continue;
        }
        let quoted = line.split('"').collect::<Vec<_>>();
        if quoted.len() < 6 {
            continue;
        }
        let project_path = root_dir.join(quoted[5].replace('\\', "/"));
        if extension_is(&project_path, "csproj") {
            projects.push(project_path);
        }
    }
    projects.sort();
    projects
}

fn attribute_value(line: &str, attribute: &str) -> Option<String> {
    let pattern = format!("{attribute}=\"");
    let start = line.find(&pattern)? + pattern.len();
    let rest = &line[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn is_solution_file(path: &Path) -> bool {
    extension_is(path, "slnx") || extension_is(path, "sln")
}

fn extension_is(path: &Path, expected: &str) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case(expected))
}
