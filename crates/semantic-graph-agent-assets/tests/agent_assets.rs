use semantic_graph_agent_assets::{
    AgentAssetsError, AssetManifest, AssetRenderer, RenderedAsset, check_assets,
    constants::{
        generated_agent_names, generated_paths,
        manifest::{DISALLOWED_EXTRACT_TOOLS_ARG, MANIFEST_PATH, MCP_SERVER_TABLE, SKILL_NAME},
        mcp::{CONFIG_ROOT_TABLE, MANAGED_SERVER_COMMAND, RESOURCE_URI_PREFIX},
        tests as test_constants, toml_fields,
    },
    generate_assets,
};
use std::{
    collections::BTreeSet,
    error::Error,
    fs,
    path::{Path, PathBuf},
    process,
    time::{SystemTime, UNIX_EPOCH},
};

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn manifest_parsing_accepts_checked_in_manifest() -> TestResult {
    let manifest = load_manifest()?;
    assert_eq!(
        expected_output_paths(),
        manifest
            .declared_output_paths()
            .into_iter()
            .map(|path| path.to_string_lossy().to_string())
            .collect::<BTreeSet<_>>()
    );
    Ok(())
}

#[test]
fn manifest_parsing_rejects_duplicate_output_paths() -> TestResult {
    let source = manifest_source()?.replacen(
        manifest_output_line(generated_paths::MCP_TOOLS_REFERENCE).as_str(),
        manifest_output_line(generated_paths::SKILL).as_str(),
        1,
    );

    let error =
        AssetManifest::from_toml_str(&source, Path::new(test_constants::MANIFEST_TEST_PATH))
            .err()
            .ok_or_else(|| boxed_error("expected duplicate output path error"))?;

    assert!(matches!(
        error,
        AgentAssetsError::DuplicateOutputPath { .. }
    ));
    assert_eq!(4, error.exit_code());
    Ok(())
}

#[test]
fn manifest_parsing_rejects_invalid_toml() -> TestResult {
    let error = AssetManifest::from_toml_str(
        test_constants::INVALID_TOML,
        Path::new(test_constants::MANIFEST_TEST_PATH),
    )
    .err()
    .ok_or_else(|| boxed_error("expected invalid TOML error"))?;

    assert!(matches!(error, AgentAssetsError::ManifestToml { .. }));
    assert_eq!(2, error.exit_code());
    Ok(())
}

#[test]
fn manifest_parsing_rejects_output_path_escapes() -> TestResult {
    let source = manifest_source()?.replacen(
        manifest_output_line(generated_paths::SKILL).as_str(),
        test_constants::OUTPUT_PATH_ESCAPE_LINE,
        1,
    );

    let error =
        AssetManifest::from_toml_str(&source, Path::new(test_constants::MANIFEST_TEST_PATH))
            .err()
            .ok_or_else(|| boxed_error("expected path escape error"))?;

    assert!(matches!(
        error,
        AgentAssetsError::OutputPathEscapesExpectedRoot { .. }
    ));
    assert_eq!(5, error.exit_code());
    Ok(())
}

#[test]
fn fragment_loading_rejects_missing_declared_fragment() -> TestResult {
    let source = manifest_source()?.replacen(
        "common/mcp-tools.md",
        test_constants::MISSING_MCP_TOOLS_FRAGMENT,
        1,
    );
    let manifest =
        AssetManifest::from_toml_str(&source, Path::new(test_constants::MANIFEST_TEST_PATH))?;

    let error = AssetRenderer::render(&repo_root()?, &manifest)
        .err()
        .ok_or_else(|| boxed_error("expected missing fragment error"))?;

    assert!(matches!(error, AgentAssetsError::MissingFragment { .. }));
    assert_eq!(3, error.exit_code());
    Ok(())
}

#[test]
fn renderer_emits_every_declared_expected_artifact() -> TestResult {
    let manifest = load_manifest()?;
    let assets = render_assets(&manifest)?;

    let declared = manifest
        .declared_output_paths()
        .into_iter()
        .map(|path| path.to_string_lossy().to_string())
        .collect::<BTreeSet<_>>();
    let rendered = assets
        .iter()
        .map(|asset| asset.output_path().to_string_lossy().to_string())
        .collect::<BTreeSet<_>>();

    assert_eq!(declared, rendered);
    Ok(())
}

#[test]
fn custom_agent_toml_parses_and_contains_required_fields() -> TestResult {
    let manifest = load_manifest()?;
    let assets = render_assets(&manifest)?;
    let mut names = BTreeSet::new();

    for asset in assets.iter().filter(|asset| {
        asset
            .output_path()
            .to_string_lossy()
            .starts_with(generated_paths::CODEX_AGENTS_DIR)
    }) {
        let value = toml::from_str::<toml::Value>(asset.content())?;
        names.insert(required_string(&value, toml_fields::NAME)?.to_string());
        assert!(!required_string(&value, toml_fields::DESCRIPTION)?.is_empty());
        assert!(!required_string(&value, toml_fields::DEVELOPER_INSTRUCTIONS)?.is_empty());
    }

    assert_eq!(
        generated_agent_names::ALL
            .iter()
            .map(|name| (*name).to_string())
            .collect::<BTreeSet<_>>(),
        names
    );
    Ok(())
}

#[test]
fn mcp_config_snippet_parses_and_uses_semantic_graph_table() -> TestResult {
    let manifest = load_manifest()?;
    let assets = render_assets(&manifest)?;
    let config = find_asset(&assets, generated_paths::CONFIG_SNIPPET)?;
    let value = toml::from_str::<toml::Value>(config.content())?;
    let server = value
        .get(CONFIG_ROOT_TABLE)
        .and_then(|value| value.get(MCP_SERVER_TABLE))
        .ok_or_else(|| {
            boxed_error(format!(
                "missing {CONFIG_ROOT_TABLE}.{MCP_SERVER_TABLE} table"
            ))
        })?;

    assert_eq!(
        Some(MANAGED_SERVER_COMMAND),
        server
            .get(toml_fields::COMMAND)
            .and_then(toml::Value::as_str)
    );
    assert_eq!(
        Some(true),
        server
            .get(toml_fields::ENABLED)
            .and_then(toml::Value::as_bool)
    );
    assert_eq!(
        Some(false),
        server
            .get(toml_fields::REQUIRED)
            .and_then(toml::Value::as_bool)
    );
    let args = server
        .get(toml_fields::ARGS)
        .and_then(toml::Value::as_array)
        .ok_or_else(|| boxed_error("missing MCP args"))?;
    assert!(args.is_empty());
    assert!(args.iter().all(|arg| {
        arg.as_str()
            .map(|value| value != DISALLOWED_EXTRACT_TOOLS_ARG)
            .unwrap_or(false)
    }));
    Ok(())
}

#[test]
fn generated_skill_contains_trigger_guidance_and_references() -> TestResult {
    let manifest = load_manifest()?;
    let assets = render_assets(&manifest)?;
    let skill = find_asset(&assets, generated_paths::SKILL)?;

    assert!(skill.content().contains(&format!("name: {SKILL_NAME}")));
    assert!(skill.content().contains("description:"));
    assert!(skill.content().contains("code relationships"));
    assert!(skill.content().contains("route freshness"));
    assert!(skill.content().contains("stale graph facts"));
    assert!(
        skill
            .content()
            .contains("Do not claim every task must query the graph first")
    );
    assert!(
        skill
            .content()
            .contains("Use node details, edge details, occurrences, and edge evidence")
    );
    assert!(skill.content().contains("references/mcp-tools.md"));
    assert!(skill.content().contains("references/rust-extraction.md"));
    assert!(skill.content().contains("references/csharp-extraction.md"));
    assert!(skill.content().contains("references/local-testbeds.md"));
    assert!(skill.content().contains("references/agent-handoffs.md"));
    assert!(skill.content().contains("references/troubleshooting.md"));
    assert!(!skill.content().contains("Always query the graph first"));
    Ok(())
}

#[test]
fn generated_references_list_all_phase_two_tools_and_resources() -> TestResult {
    let manifest = load_manifest()?;
    let assets = render_assets(&manifest)?;
    let mcp_tools = find_asset(&assets, generated_paths::MCP_TOOLS_REFERENCE)?;

    for tool_name in phase_two_tool_names()? {
        assert!(
            mcp_tools.content().contains(&format!("`{tool_name}`")),
            "missing tool {tool_name}"
        );
    }

    for resource_uri in phase_two_resource_uris()? {
        assert!(
            mcp_tools.content().contains(&format!("`{resource_uri}`")),
            "missing resource {resource_uri}"
        );
    }

    Ok(())
}

#[test]
fn generated_assets_do_not_include_smoke_agent_or_incompatible_handoffs() -> TestResult {
    let manifest = load_manifest()?;
    let assets = render_assets(&manifest)?;

    for asset in &assets {
        let path = asset.output_path().to_string_lossy();
        assert!(!path.contains("smoke"));
        assert!(!asset.content().contains("smoke-test custom agent"));
        assert!(!asset.content().contains("chunk"));
        assert!(!asset.content().contains("Claude"));
    }

    Ok(())
}

#[test]
fn local_testbed_reference_keeps_visualizer_as_prior_art() -> TestResult {
    let manifest = load_manifest()?;
    let assets = render_assets(&manifest)?;
    let local_testbeds = find_asset(&assets, generated_paths::LOCAL_TESTBEDS_REFERENCE)?;

    assert!(local_testbeds.content().contains("optional prior art"));
    assert!(
        local_testbeds
            .content()
            .contains("not the durable MCP boundary")
    );
    assert!(
        local_testbeds
            .content()
            .contains("do not define MCP tool or resource names")
    );
    Ok(())
}

#[test]
fn generate_writes_expected_tree_in_temp_copy() -> TestResult {
    let temp = create_temp_repo(test_constants::GENERATE_WRITES_TEMP_NAME)?;
    let report = generate_assets(&temp)?;

    assert_eq!(14, report.created.len());
    assert!(
        temp.join(test_constants::GENERATED_EXPECTED_ROOT)
            .join(generated_paths::SKILL)
            .exists()
    );
    cleanup_temp_repo(&temp)?;
    Ok(())
}

#[test]
fn check_passes_against_matching_generated_expected_artifacts() -> TestResult {
    let temp = create_temp_repo(test_constants::CHECK_PASSES_TEMP_NAME)?;
    generate_assets(&temp)?;
    let report = check_assets(&temp)?;

    assert_eq!(14, report.checked.len());
    cleanup_temp_repo(&temp)?;
    Ok(())
}

#[test]
fn check_fails_when_expected_file_is_changed() -> TestResult {
    let temp = create_temp_repo(test_constants::CHECK_CHANGED_TEMP_NAME)?;
    generate_assets(&temp)?;
    fs::write(
        temp.join(test_constants::GENERATED_EXPECTED_ROOT)
            .join(generated_paths::README),
        "changed\n",
    )?;

    let error = check_assets(&temp)
        .err()
        .ok_or_else(|| boxed_error("expected changed artifact drift"))?;

    assert!(matches!(error, AgentAssetsError::Drift { .. }));
    assert_eq!(6, error.exit_code());
    assert!(
        error
            .user_message()
            .contains(&format!("changed: {}", generated_paths::README))
    );
    cleanup_temp_repo(&temp)?;
    Ok(())
}

#[test]
fn check_fails_when_expected_file_is_missing() -> TestResult {
    let temp = create_temp_repo(test_constants::CHECK_MISSING_TEMP_NAME)?;
    generate_assets(&temp)?;
    fs::remove_file(
        temp.join(test_constants::GENERATED_EXPECTED_ROOT)
            .join(generated_paths::SKILL),
    )?;

    let error = check_assets(&temp)
        .err()
        .ok_or_else(|| boxed_error("expected missing artifact drift"))?;

    assert!(matches!(error, AgentAssetsError::Drift { .. }));
    assert_eq!(6, error.exit_code());
    assert!(
        error
            .user_message()
            .contains(&format!("missing: {}", generated_paths::SKILL))
    );
    cleanup_temp_repo(&temp)?;
    Ok(())
}

#[test]
fn check_fails_when_stale_expected_file_exists() -> TestResult {
    let temp = create_temp_repo(test_constants::CHECK_STALE_TEMP_NAME)?;
    generate_assets(&temp)?;
    fs::write(
        temp.join(test_constants::GENERATED_EXPECTED_ROOT)
            .join(test_constants::STALE_ARTIFACT),
        "stale = true\n",
    )?;

    let error = check_assets(&temp)
        .err()
        .ok_or_else(|| boxed_error("expected stale artifact drift"))?;

    assert!(matches!(error, AgentAssetsError::Drift { .. }));
    assert_eq!(6, error.exit_code());
    assert!(
        error
            .user_message()
            .contains(&format!("stale: {}", test_constants::STALE_ARTIFACT))
    );
    cleanup_temp_repo(&temp)?;
    Ok(())
}

#[test]
fn check_rejects_duplicate_output_paths() -> TestResult {
    let temp = create_temp_repo(test_constants::CHECK_DUPLICATE_TEMP_NAME)?;
    replace_manifest(
        &temp,
        manifest_output_line(generated_paths::MCP_TOOLS_REFERENCE).as_str(),
        manifest_output_line(generated_paths::SKILL).as_str(),
    )?;

    let error = check_assets(&temp)
        .err()
        .ok_or_else(|| boxed_error("expected duplicate output path error"))?;

    assert!(matches!(
        error,
        AgentAssetsError::DuplicateOutputPath { .. }
    ));
    assert_eq!(4, error.exit_code());
    cleanup_temp_repo(&temp)?;
    Ok(())
}

#[test]
fn check_rejects_output_paths_outside_expected_root() -> TestResult {
    let temp = create_temp_repo(test_constants::CHECK_PATH_ESCAPE_TEMP_NAME)?;
    replace_manifest(
        &temp,
        manifest_output_line(generated_paths::SKILL).as_str(),
        test_constants::OUTPUT_PATH_ESCAPE_LINE,
    )?;

    let error = check_assets(&temp)
        .err()
        .ok_or_else(|| boxed_error("expected output path escape error"))?;

    assert!(matches!(
        error,
        AgentAssetsError::OutputPathEscapesExpectedRoot { .. }
    ));
    assert_eq!(5, error.exit_code());
    cleanup_temp_repo(&temp)?;
    Ok(())
}

#[test]
fn generate_rejects_output_paths_outside_expected_root() -> TestResult {
    let temp = create_temp_repo(test_constants::GENERATE_PATH_ESCAPE_TEMP_NAME)?;
    replace_manifest(
        &temp,
        manifest_output_line(generated_paths::SKILL).as_str(),
        test_constants::OUTPUT_PATH_ESCAPE_LINE,
    )?;

    let error = generate_assets(&temp)
        .err()
        .ok_or_else(|| boxed_error("expected output path escape error"))?;

    assert!(matches!(
        error,
        AgentAssetsError::OutputPathEscapesExpectedRoot { .. }
    ));
    assert_eq!(5, error.exit_code());
    cleanup_temp_repo(&temp)?;
    Ok(())
}

fn load_manifest() -> Result<AssetManifest, Box<dyn Error>> {
    AssetManifest::load_from_repo(&repo_root()?).map_err(Into::into)
}

fn render_assets(manifest: &AssetManifest) -> Result<Vec<RenderedAsset>, Box<dyn Error>> {
    AssetRenderer::render(&repo_root()?, manifest).map_err(Into::into)
}

fn manifest_source() -> Result<String, Box<dyn Error>> {
    let path = repo_root()?.join(MANIFEST_PATH);
    fs::read_to_string(path).map_err(Into::into)
}

fn repo_root() -> Result<PathBuf, Box<dyn Error>> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let crates_dir = manifest_dir
        .parent()
        .ok_or_else(|| boxed_error("crate directory has no parent"))?;
    let repo_root = crates_dir
        .parent()
        .ok_or_else(|| boxed_error("crates directory has no parent"))?;
    Ok(repo_root.to_path_buf())
}

fn find_asset<'a>(
    assets: &'a [RenderedAsset],
    path: &str,
) -> Result<&'a RenderedAsset, Box<dyn Error>> {
    assets
        .iter()
        .find(|asset| asset.output_path().to_string_lossy() == path)
        .ok_or_else(|| boxed_error(format!("missing rendered asset {path}")))
}

fn required_string<'a>(value: &'a toml::Value, key: &str) -> Result<&'a str, Box<dyn Error>> {
    value
        .get(key)
        .and_then(toml::Value::as_str)
        .ok_or_else(|| boxed_error(format!("missing string field {key}")))
}

fn phase_two_tool_names() -> Result<Vec<String>, Box<dyn Error>> {
    let source = fs::read_to_string(repo_root()?.join(test_constants::MCP_TOOL_REGISTRY_SOURCE))?;
    quoted_const_values(&source, "pub const GRAPH_")
}

fn phase_two_resource_uris() -> Result<Vec<String>, Box<dyn Error>> {
    let mut values = Vec::new();
    for source_path in test_constants::RESOURCE_SOURCE_FILES {
        let source = fs::read_to_string(repo_root()?.join(source_path))?;
        values.extend(quoted_const_values(&source, "pub const ")?);
    }
    Ok(values
        .into_iter()
        .filter(|value| value.starts_with(RESOURCE_URI_PREFIX))
        .collect())
}

fn quoted_const_values(source: &str, prefix: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let mut values = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with(prefix) {
            values.push(quoted_value(trimmed)?);
        }
    }
    Ok(values)
}

fn quoted_value(line: &str) -> Result<String, Box<dyn Error>> {
    let start = line
        .find('"')
        .ok_or_else(|| boxed_error(format!("missing opening quote in {line}")))?;
    let rest = &line[start + 1..];
    let end = rest
        .find('"')
        .ok_or_else(|| boxed_error(format!("missing closing quote in {line}")))?;
    Ok(rest[..end].to_string())
}

fn expected_output_paths() -> BTreeSet<String> {
    generated_paths::ALL
        .iter()
        .map(|path| (*path).to_string())
        .collect()
}

fn create_temp_repo(name: &str) -> Result<PathBuf, Box<dyn Error>> {
    let root = unique_temp_root(name)?;
    let manifest_path = Path::new(MANIFEST_PATH);
    let asset_root = manifest_path
        .parent()
        .ok_or_else(|| boxed_error("manifest path has no parent"))?;
    fs::create_dir_all(root.join(asset_root))?;
    fs::copy(repo_root()?.join(MANIFEST_PATH), root.join(MANIFEST_PATH))?;
    copy_dir(
        &repo_root()?.join(test_constants::FRAGMENTS_PATH),
        &root.join(test_constants::FRAGMENTS_PATH),
    )?;
    Ok(root)
}

fn unique_temp_root(name: &str) -> Result<PathBuf, Box<dyn Error>> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    for attempt in 0..100u32 {
        let path = std::env::temp_dir().join(format!(
            "{}-{name}-{}-{nanos}-{attempt}",
            test_constants::TEMP_REPO_DIR_PREFIX,
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

fn copy_dir(source: &Path, destination: &Path) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            copy_dir(&source_path, &destination_path)?;
        } else {
            fs::copy(source_path, destination_path)?;
        }
    }
    Ok(())
}

fn replace_manifest(root: &Path, from: &str, to: &str) -> Result<(), Box<dyn Error>> {
    let path = root.join(MANIFEST_PATH);
    let source = fs::read_to_string(&path)?;
    fs::write(path, source.replacen(from, to, 1))?;
    Ok(())
}

fn cleanup_temp_repo(root: &Path) -> Result<(), Box<dyn Error>> {
    fs::remove_dir_all(root).map_err(Into::into)
}

fn boxed_error(message: impl Into<String>) -> Box<dyn Error> {
    Box::new(std::io::Error::other(message.into()))
}

fn manifest_output_line(output_path: &str) -> String {
    format!("output_path = \"{output_path}\"")
}
