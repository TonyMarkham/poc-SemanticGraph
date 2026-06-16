use crate::{CSharpLsLibError, CSharpLsLibResult, model::CSharpSolutionModel};

use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn project_source_files(
    model: &CSharpSolutionModel,
    project_or_root: &Path,
) -> CSharpLsLibResult<Vec<PathBuf>> {
    let boundary = project_or_root
        .canonicalize()
        .map_err(|source| CSharpLsLibError::io("canonicalize C# project boundary", None, source))?;
    let mut files = Vec::new();
    for project in &model.projects {
        if project.project_path == boundary || project.project_path.starts_with(&boundary) {
            files.extend(project.source_files.iter().cloned());
        }
    }
    files.sort();
    files.dedup();
    if files.is_empty() {
        return Err(CSharpLsLibError::invalid_path(
            boundary,
            "C# project boundary matched no source files in the resolved solution",
        ));
    }

    Ok(files)
}

pub(crate) fn project_file_source_files(project_path: &Path) -> CSharpLsLibResult<Vec<PathBuf>> {
    let project_path = project_path
        .canonicalize()
        .map_err(|source| CSharpLsLibError::io("canonicalize C# project path", None, source))?;
    let project_dir = project_path
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| {
            CSharpLsLibError::invalid_path(&project_path, "project path has no parent directory")
        })?;
    let contents = fs::read_to_string(&project_path).map_err(|source| {
        CSharpLsLibError::io("read C# project file", Some(project_path.clone()), source)
    })?;

    let mut files = explicit_compile_items(&project_dir, &contents);
    if files.is_empty() || is_sdk_style_project(&contents) {
        collect_default_compile_items(&project_dir, &mut files)?;
    }
    files.sort();
    files.dedup();
    if files.is_empty() {
        return Err(CSharpLsLibError::response_shape(
            "project_source_files",
            format!(
                "project {} contained no C# source files",
                project_path.display()
            ),
        ));
    }

    Ok(files)
}

fn explicit_compile_items(project_dir: &Path, contents: &str) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for line in contents.lines() {
        if !line.contains("<Compile") {
            continue;
        }
        let Some(include) = attribute_value(line, "Include") else {
            continue;
        };
        if include.contains('*') {
            continue;
        }
        let file_path = project_dir.join(include);
        if is_csharp_source_file(&file_path) && !is_excluded_path(&file_path) {
            files.push(file_path);
        }
    }
    files
}

fn is_sdk_style_project(contents: &str) -> bool {
    contents.contains("<Project Sdk=") || contents.contains("<Project sdk=")
}

fn collect_default_compile_items(root: &Path, files: &mut Vec<PathBuf>) -> CSharpLsLibResult<()> {
    for entry in fs::read_dir(root).map_err(|source| {
        CSharpLsLibError::io(
            "read C# project source directory",
            Some(root.to_path_buf()),
            source,
        )
    })? {
        let entry = entry.map_err(|source| {
            CSharpLsLibError::io(
                "read C# project directory entry",
                Some(root.to_path_buf()),
                source,
            )
        })?;
        let path = entry.path();
        if is_excluded_path(&path) {
            continue;
        }
        if path.is_dir() {
            collect_default_compile_items(&path, files)?;
        } else if is_csharp_source_file(&path) {
            files.push(path);
        }
    }

    Ok(())
}

fn attribute_value(line: &str, attribute: &str) -> Option<String> {
    let pattern = format!("{attribute}=\"");
    let start = line.find(&pattern)? + pattern.len();
    let rest = &line[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn is_csharp_source_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("cs"))
}

fn is_excluded_path(path: &Path) -> bool {
    path.components().any(|component| {
        let name = component.as_os_str().to_string_lossy();
        name == "bin"
            || name == "obj"
            || name == ".git"
            || name.starts_with('.')
            || name.ends_with(".g.cs")
            || name.ends_with(".g.i.cs")
            || name.ends_with(".Designer.cs")
    })
}
