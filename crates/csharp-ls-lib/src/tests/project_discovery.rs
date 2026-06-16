use crate::{load_solution, project_for_file, project_source_files, solution_source_files};

use std::{
    error::Error,
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn loads_slnx_projects_and_discovers_sdk_style_sources() -> Result<(), Box<dyn Error>> {
    let root = temp_dir("loads-slnx")?;
    let project_dir = root.join("Project");
    fs::create_dir_all(&project_dir)?;
    let solution_path = root.join("Demo.slnx");
    let project_path = project_dir.join("Project.csproj");
    let source_path = project_dir.join("Program.cs");
    let ignored_path = project_dir.join("obj/Generated.g.cs");
    fs::create_dir_all(ignored_path.parent().ok_or("expected ignored parent")?)?;
    fs::write(
        &solution_path,
        r#"<Solution>
  <Folder Name="/Project/">
    <Project Path="Project/Project.csproj" />
  </Folder>
</Solution>
"#,
    )?;
    fs::write(&project_path, r#"<Project Sdk="Microsoft.NET.Sdk" />"#)?;
    fs::write(&source_path, "class Program {}")?;
    fs::write(&ignored_path, "class Generated {}")?;

    let model = load_solution(&solution_path)?;
    let files = solution_source_files(&model);

    assert_eq!(files, vec![source_path.canonicalize()?]);
    assert_eq!(
        project_for_file(&model, &source_path)?.project_path,
        project_path.canonicalize()?
    );
    assert_eq!(
        project_source_files(&model, &project_path)?,
        vec![source_path.canonicalize()?]
    );
    Ok(())
}

#[test]
fn loads_sln_project_entries() -> Result<(), Box<dyn Error>> {
    let root = temp_dir("loads-sln")?;
    let project_dir = root.join("Project");
    fs::create_dir_all(&project_dir)?;
    let solution_path = root.join("Demo.sln");
    let project_path = project_dir.join("Project.csproj");
    let source_path = project_dir.join("Program.cs");
    fs::write(
        &solution_path,
        r#"Project("{GUID}") = "Project", "Project/Project.csproj", "{PROJECT-GUID}"
EndProject
"#,
    )?;
    fs::write(&project_path, r#"<Project Sdk="Microsoft.NET.Sdk" />"#)?;
    fs::write(&source_path, "class Program {}")?;

    let model = load_solution(&solution_path)?;

    assert_eq!(
        solution_source_files(&model),
        vec![source_path.canonicalize()?]
    );
    Ok(())
}

fn temp_dir(name: &str) -> Result<PathBuf, Box<dyn Error>> {
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let path = std::env::temp_dir().join(format!(
        "csharp-ls-lib-{name}-{}-{stamp}",
        std::process::id()
    ));
    fs::create_dir_all(&path)?;
    Ok(path)
}
