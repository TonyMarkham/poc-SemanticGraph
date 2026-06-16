use clap::Parser;
use semantic_graph_agent_assets::constants::generated_paths;
use semantic_graph_cli::{
    AssetSource, Checksum, CommandOutput, FileActionKind, InstallManifest, InstallManifestMode,
    ManagedFileManifestEntry, McpInstallMode, SemanticGraphArgs, SemanticGraphCliError,
    run_with_args,
};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    error::Error,
    fmt::Write,
    fs,
    path::{Path, PathBuf},
    process::{self, Command},
    time::{SystemTime, UNIX_EPOCH},
};

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn cli_parsing_accepts_install_codex() -> TestResult {
    let args = SemanticGraphArgs::try_parse_from(["semantic-graph", "install", "codex"])?;

    let semantic_graph_cli::SemanticGraphCommand::Install(install_args) = args.command;
    let semantic_graph_cli::InstallCommand::Codex(codex_args) = install_args.command;
    assert_eq!(Path::new("."), codex_args.project());
    assert_eq!(
        semantic_graph_cli::McpInstallMode::ReadOnly,
        codex_args.mcp()
    );
    assert_eq!(None, codex_args.database_path());
    assert!(!codex_args.dry_run());
    assert!(!codex_args.force());
    Ok(())
}

#[test]
fn cli_parsing_accepts_supported_install_options() -> TestResult {
    let args = SemanticGraphArgs::try_parse_from([
        "semantic-graph",
        "install",
        "codex",
        "--project",
        "project-a",
        "--database-path",
        ".local/graph.db",
        "--mcp",
        "disabled",
        "--dry-run",
        "--force",
    ])?;

    let semantic_graph_cli::SemanticGraphCommand::Install(install_args) = args.command;
    let semantic_graph_cli::InstallCommand::Codex(codex_args) = install_args.command;
    assert_eq!(Path::new("project-a"), codex_args.project());
    assert_eq!(Some(".local/graph.db"), codex_args.database_path());
    assert_eq!(
        semantic_graph_cli::McpInstallMode::Disabled,
        codex_args.mcp()
    );
    assert!(codex_args.dry_run());
    assert!(codex_args.force());
    Ok(())
}

#[test]
fn cli_parsing_rejects_unsupported_commands_and_modes() {
    assert!(SemanticGraphArgs::try_parse_from(["semantic-graph", "doctor"]).is_err());
    assert!(SemanticGraphArgs::try_parse_from(["semantic-graph", "install", "uninstall"]).is_err());
    assert!(
        SemanticGraphArgs::try_parse_from([
            "semantic-graph",
            "install",
            "codex",
            "--mcp",
            "extract"
        ])
        .is_err()
    );
    assert!(
        SemanticGraphArgs::try_parse_from([
            "semantic-graph",
            "install",
            "codex",
            "--enable-extract-tools"
        ])
        .is_err()
    );
}

#[test]
fn project_path_validation_rejects_output_path_escapes() -> TestResult {
    let project = temp_dir("path-escape")?;
    let root = semantic_graph_cli::ProjectRoot::resolve(&project, Path::new("."))?;

    let error = root
        .target_path(Path::new("../escape.md"))
        .err()
        .ok_or_else(|| boxed_error("expected path escape error"))?;

    assert!(matches!(
        error,
        SemanticGraphCliError::InvalidInstallPath { .. }
    ));
    let absolute_error = root
        .target_path(Path::new("/tmp/semantic-graph-escape.md"))
        .err()
        .ok_or_else(|| boxed_error("expected absolute path error"))?;
    assert!(matches!(
        absolute_error,
        SemanticGraphCliError::InvalidInstallPath { .. }
    ));
    cleanup(&project)?;
    Ok(())
}

#[test]
fn checksum_generation_is_stable_for_known_content() {
    assert_eq!(
        "e9e7c0748a85d2e4db2b314302c993e75505320e91514195b7dae5ade379b902",
        Checksum::sha256_hex(b"semantic-graph\n")
    );
}

#[test]
fn fresh_install_writes_expected_project_local_files() -> TestResult {
    let project = temp_dir("fresh-install")?;
    let report = run_install(&project, &[])?;

    assert_eq!(15, report.actions().len());
    assert_eq!(
        expected_installed_paths()
            .into_iter()
            .map(|path| path.to_string_lossy().to_string())
            .collect::<BTreeSet<_>>(),
        report
            .actions()
            .iter()
            .map(|action| action.relative_path().to_string_lossy().to_string())
            .collect::<BTreeSet<_>>()
    );
    for path in expected_installed_paths() {
        assert!(
            project.join(&path).exists(),
            "missing installed file {}",
            path.display()
        );
    }
    assert!(!project.join(generated_paths::CONFIG_SNIPPET).exists());
    assert!(
        all_files_under(&project)?
            .iter()
            .all(|path| path.starts_with(&project))
    );

    let manifest = read_manifest(&project)?;
    assert_eq!(1, manifest.schema_version);
    assert_eq!("semantic-graph-cli", manifest.installer_crate);
    assert_eq!(
        semantic_graph_cli::McpInstallMode::ReadOnly,
        manifest.mode.mcp
    );
    assert_eq!(None, manifest.mode.database_path);
    assert_eq!(
        "agent-assets/manifest.toml",
        manifest.asset_source.manifest_path
    );
    assert_eq!(
        "semantic-graph-agent-assets",
        manifest.asset_source.asset_generation
    );
    assert_eq!(14, manifest.managed_files.len());
    assert_manifest_checksums(&project, &manifest)?;

    let server = semantic_graph_server_config(&project)?;
    assert_eq!(
        Some(true),
        server.get("enabled").and_then(toml::Value::as_bool)
    );
    assert!(
        server
            .get("args")
            .and_then(toml::Value::as_array)
            .ok_or_else(|| boxed_error("missing args array"))?
            .is_empty()
    );
    cleanup(&project)?;
    Ok(())
}

#[test]
fn binary_dry_run_prints_plan_without_writing_files() -> TestResult {
    let project = temp_dir("binary-dry-run")?;
    let output = Command::new(env!("CARGO_BIN_EXE_semantic-graph"))
        .args([
            "install",
            "codex",
            "--project",
            &project.display().to_string(),
            "--dry-run",
        ])
        .output()?;

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("dry-run"));
    assert!(stdout.contains("created: .agents/skills/semantic-graph/SKILL.md"));
    assert!(stdout.contains("created: .codex/semantic-graph/install-manifest.json"));
    assert!(!project.join(".agents").exists());
    assert!(!project.join(".codex").exists());
    cleanup(&project)?;
    Ok(())
}

#[test]
fn dry_run_reports_plan_without_writing_files() -> TestResult {
    let project = temp_dir("dry-run")?;
    let report = run_install(&project, &["--dry-run"])?;

    assert_eq!(15, report.actions().len());
    assert!(report.lines()[0].contains("no files written"));
    assert!(!project.join(".agents").exists());
    assert!(!project.join(".codex").exists());
    cleanup(&project)?;
    Ok(())
}

#[test]
fn second_install_skips_identical_managed_files() -> TestResult {
    let project = temp_dir("skip-identical")?;
    run_install(&project, &[])?;
    let report = run_install(&project, &[])?;

    for path in expected_non_manifest_managed_paths() {
        assert_eq!(
            Some(FileActionKind::Skip),
            action_for(&report, &path),
            "{}",
            path.display()
        );
    }
    assert_eq!(
        Some(FileActionKind::Update),
        action_for(
            &report,
            Path::new(".codex/semantic-graph/install-manifest.json")
        )
    );
    cleanup(&project)?;
    Ok(())
}

#[test]
fn existing_manifest_authorizes_managed_file_update() -> TestResult {
    let project = temp_dir("manifest-authorized-update")?;
    let skill_path = project.join(generated_paths::SKILL);
    let old_content = "old managed skill\n";
    let parent = skill_path
        .parent()
        .ok_or_else(|| boxed_error("skill path has no parent"))?;
    fs::create_dir_all(parent)?;
    fs::write(&skill_path, old_content)?;
    write_manifest_for_file(
        &project,
        generated_paths::SKILL,
        sha256_hex(old_content.as_bytes()),
    )?;

    let report = run_install(&project, &[])?;

    assert_eq!(
        Some(FileActionKind::Update),
        action_for(&report, Path::new(generated_paths::SKILL))
    );
    assert_ne!(old_content, fs::read_to_string(skill_path)?);
    cleanup(&project)?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn install_refuses_symlink_escape_paths() -> TestResult {
    let root = temp_dir("symlink-root")?;
    let project = root.join("project");
    let outside = root.join("outside");
    fs::create_dir_all(&project)?;
    fs::create_dir_all(&outside)?;
    std::os::unix::fs::symlink(&outside, project.join(".agents"))?;

    let error = run_install_error(&project, &[])?;

    assert!(matches!(
        error,
        SemanticGraphCliError::InvalidInstallPath { .. }
    ));
    assert!(fs::read_dir(&outside)?.next().is_none());
    cleanup(&root)?;
    Ok(())
}

#[test]
fn disabled_mcp_writes_disabled_project_config_table() -> TestResult {
    let project = temp_dir("disabled-mcp")?;
    run_install(&project, &["--mcp", "disabled"])?;

    let server = semantic_graph_server_config(&project)?;
    assert_eq!(
        Some(false),
        server.get("enabled").and_then(toml::Value::as_bool)
    );
    assert_no_extract_tools(&server)?;
    cleanup(&project)?;
    Ok(())
}

#[test]
fn database_path_is_recorded_in_mcp_args() -> TestResult {
    let project = temp_dir("database-path")?;
    run_install(&project, &["--database-path", ".local/rust-workspace.db"])?;

    let server = semantic_graph_server_config(&project)?;
    let args = server
        .get("args")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| boxed_error("missing args array"))?;
    assert_eq!(
        vec!["--database-path", ".local/rust-workspace.db"],
        args.iter()
            .map(|value| value.as_str().unwrap_or_default())
            .collect::<Vec<_>>()
    );
    assert_no_extract_tools(&server)?;
    let manifest = read_manifest(&project)?;
    assert_eq!(
        Some(".local/rust-workspace.db".to_string()),
        manifest.mode.database_path
    );
    cleanup(&project)?;
    Ok(())
}

#[test]
fn install_preserves_unrelated_codex_config_content() -> TestResult {
    let project = temp_dir("preserve-config")?;
    fs::create_dir_all(project.join(".codex"))?;
    fs::write(
        project.join(".codex/config.toml"),
        r#"model = "gpt-5"

[mcp_servers.other]
command = "other-server"
enabled = true

[mcp_servers.semantic_graph]
command = "old"
custom = "keep-me"
"#,
    )?;

    run_install(&project, &[])?;

    let config = codex_config(&project)?;
    assert_eq!(
        Some("gpt-5"),
        config.get("model").and_then(toml::Value::as_str)
    );
    assert_eq!(
        Some("other-server"),
        config
            .get("mcp_servers")
            .and_then(toml::Value::as_table)
            .and_then(|servers| servers.get("other"))
            .and_then(toml::Value::as_table)
            .and_then(|server| server.get("command"))
            .and_then(toml::Value::as_str)
    );
    let server = semantic_graph_server_config(&project)?;
    assert_eq!(
        Some(".refactor-radar/bin/semantic-graph-mcp-server"),
        server.get("command").and_then(toml::Value::as_str)
    );
    assert_eq!(
        Some("keep-me"),
        server.get("custom").and_then(toml::Value::as_str)
    );
    cleanup(&project)?;
    Ok(())
}

#[test]
fn install_refuses_user_authored_generated_file_by_default() -> TestResult {
    let project = temp_dir("conflict")?;
    let skill_path = project.join(generated_paths::SKILL);
    let parent = skill_path
        .parent()
        .ok_or_else(|| boxed_error("skill path has no parent"))?;
    fs::create_dir_all(parent)?;
    fs::write(&skill_path, "user-authored\n")?;

    let error = run_install_error(&project, &[])?;

    assert!(matches!(error, SemanticGraphCliError::RefusedWrites { .. }));
    assert!(
        error
            .user_message()
            .contains(&format!("refused: {}", generated_paths::SKILL))
    );
    assert_eq!("user-authored\n", fs::read_to_string(&skill_path)?);
    assert!(
        !project
            .join(".codex/semantic-graph/install-manifest.json")
            .exists()
    );
    cleanup(&project)?;
    Ok(())
}

#[test]
fn force_replaces_only_managed_conflict_paths() -> TestResult {
    let project = temp_dir("force")?;
    let skill_path = project.join(generated_paths::SKILL);
    let unrelated_path = project.join("notes.md");
    let parent = skill_path
        .parent()
        .ok_or_else(|| boxed_error("skill path has no parent"))?;
    fs::create_dir_all(parent)?;
    fs::write(&skill_path, "user-authored\n")?;
    fs::write(&unrelated_path, "leave alone\n")?;

    let report = run_install(&project, &["--force"])?;

    assert_eq!(
        Some(FileActionKind::Update),
        action_for(&report, Path::new(generated_paths::SKILL))
    );
    assert_ne!("user-authored\n", fs::read_to_string(&skill_path)?);
    assert_eq!("leave alone\n", fs::read_to_string(unrelated_path)?);
    cleanup(&project)?;
    Ok(())
}

#[test]
fn manifest_checksums_match_installed_files() -> TestResult {
    let project = temp_dir("manifest-checksums")?;
    run_install(&project, &[])?;

    let manifest_source =
        fs::read_to_string(project.join(".codex/semantic-graph/install-manifest.json"))?;
    let manifest = read_manifest(&project)?;
    assert_manifest_checksums(&project, &manifest)?;
    let mut serialized_once = serde_json::to_string_pretty(&manifest)?;
    serialized_once.push('\n');
    let mut serialized_twice = serde_json::to_string_pretty(&manifest)?;
    serialized_twice.push('\n');
    assert_eq!(serialized_once, serialized_twice);
    assert_eq!(manifest_source, serialized_once);
    assert!(
        manifest
            .managed_files
            .iter()
            .all(|entry| { entry.path != ".codex/semantic-graph/install-manifest.json" })
    );
    cleanup(&project)?;
    Ok(())
}

fn run_install(
    project: &Path,
    extra: &[&str],
) -> Result<semantic_graph_cli::CodexInstallReport, Box<dyn Error>> {
    let args = SemanticGraphArgs::try_parse_from(install_command(project, extra))?;
    let output = run_with_args(args)?;
    match output {
        CommandOutput::InstallCodex(report) => Ok(report),
    }
}

fn run_install_error(
    project: &Path,
    extra: &[&str],
) -> Result<SemanticGraphCliError, Box<dyn Error>> {
    let args = SemanticGraphArgs::try_parse_from(install_command(project, extra))?;
    match run_with_args(args) {
        Ok(_) => Err(boxed_error("expected install error")),
        Err(error) => Ok(error),
    }
}

fn install_command(project: &Path, extra: &[&str]) -> Vec<String> {
    let mut args = vec![
        "semantic-graph".to_string(),
        "install".to_string(),
        "codex".to_string(),
        "--project".to_string(),
        project.display().to_string(),
    ];
    args.extend(extra.iter().map(|value| (*value).to_string()));
    args
}

fn expected_installed_paths() -> Vec<PathBuf> {
    let mut paths = generated_paths::ALL
        .iter()
        .filter(|path| **path != generated_paths::CONFIG_SNIPPET)
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    paths.push(PathBuf::from(".codex/config.toml"));
    paths.push(PathBuf::from(".codex/semantic-graph/install-manifest.json"));
    paths.sort();
    paths
}

fn expected_non_manifest_managed_paths() -> Vec<PathBuf> {
    expected_installed_paths()
        .into_iter()
        .filter(|path| path != Path::new(".codex/semantic-graph/install-manifest.json"))
        .collect()
}

fn codex_config(project: &Path) -> Result<toml::Value, Box<dyn Error>> {
    let source = fs::read_to_string(project.join(".codex/config.toml"))?;
    toml::from_str::<toml::Value>(&source).map_err(Into::into)
}

fn semantic_graph_server_config(project: &Path) -> Result<toml::Table, Box<dyn Error>> {
    let value = codex_config(project)?;
    value
        .get("mcp_servers")
        .and_then(toml::Value::as_table)
        .and_then(|servers| servers.get("semantic_graph"))
        .and_then(toml::Value::as_table)
        .cloned()
        .ok_or_else(|| boxed_error("missing semantic_graph server table"))
}

fn assert_no_extract_tools(server: &toml::Table) -> TestResult {
    let args = server
        .get("args")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| boxed_error("missing args array"))?;
    assert!(args.iter().all(|value| {
        value
            .as_str()
            .map(|arg| arg != "--enable-extract-tools")
            .unwrap_or(true)
    }));
    Ok(())
}

fn read_manifest(project: &Path) -> Result<InstallManifest, Box<dyn Error>> {
    let source = fs::read_to_string(project.join(".codex/semantic-graph/install-manifest.json"))?;
    serde_json::from_str::<InstallManifest>(&source).map_err(Into::into)
}

fn write_manifest_for_file(project: &Path, relative_path: &str, sha256: String) -> TestResult {
    let manifest = InstallManifest {
        schema_version: 1,
        installer_crate: "semantic-graph-cli".to_string(),
        installer_version: env!("CARGO_PKG_VERSION").to_string(),
        project_root: project.display().to_string(),
        mode: InstallManifestMode::new(McpInstallMode::ReadOnly, None),
        asset_source: AssetSource::new("agent-assets/manifest.toml", "semantic-graph-agent-assets"),
        managed_files: vec![ManagedFileManifestEntry::new(
            relative_path.to_string(),
            sha256,
            FileActionKind::Update,
        )],
    };
    let path = project.join(".codex/semantic-graph/install-manifest.json");
    let parent = path
        .parent()
        .ok_or_else(|| boxed_error("install manifest path has no parent"))?;
    fs::create_dir_all(parent)?;
    let mut source = serde_json::to_string_pretty(&manifest)?;
    source.push('\n');
    fs::write(path, source)?;
    Ok(())
}

fn assert_manifest_checksums(project: &Path, manifest: &InstallManifest) -> TestResult {
    for entry in &manifest.managed_files {
        let content = fs::read(project.join(&entry.path))?;
        assert_eq!(sha256_hex(&content), entry.sha256, "{}", entry.path);
    }
    Ok(())
}

fn action_for(
    report: &semantic_graph_cli::CodexInstallReport,
    relative_path: &Path,
) -> Option<FileActionKind> {
    report
        .actions()
        .iter()
        .find(|action| action.relative_path() == relative_path)
        .map(semantic_graph_cli::FileAction::kind)
}

fn all_files_under(root: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut files = Vec::new();
    collect_files(root, &mut files)?;
    Ok(files)
}

fn collect_files(root: &Path, files: &mut Vec<PathBuf>) -> TestResult {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            collect_files(&path, files)?;
        } else {
            files.push(path);
        }
    }
    Ok(())
}

fn temp_dir(name: &str) -> Result<PathBuf, Box<dyn Error>> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    for attempt in 0..100u32 {
        let path = std::env::temp_dir().join(format!(
            "semantic-graph-cli-{name}-{}-{nanos}-{attempt}",
            process::id()
        ));
        match fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(source) if source.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(source) => return Err(Box::new(source)),
        }
    }
    Err(boxed_error("could not create unique temporary directory"))
}

fn cleanup(path: &Path) -> TestResult {
    fs::remove_dir_all(path).map_err(Into::into)
}

fn sha256_hex(content: &[u8]) -> String {
    let digest = Sha256::digest(content);
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        let _ = write!(output, "{byte:02x}");
    }
    output
}

fn boxed_error(message: impl Into<String>) -> Box<dyn Error> {
    Box::new(std::io::Error::other(message.into()))
}
