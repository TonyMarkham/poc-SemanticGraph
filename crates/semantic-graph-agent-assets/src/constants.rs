pub mod cli {
    pub const CHECK_COMMAND_NAME: &str = "check";
    pub const COMMAND_NAME: &str = "semantic-graph-agent-assets";
    pub const GENERATE_COMMAND_NAME: &str = "generate";
}

pub mod manifest {
    pub const MANIFEST_PATH: &str = "agent-assets/manifest.toml";
    pub const EXPECTED_ROOT_PREFIX: &str = "agent-assets/expected";
    pub const HOST_CODEX: &str = "codex";
    pub const SKILL_NAME: &str = "semantic-graph";
    pub const MCP_SERVER_TABLE: &str = "semantic_graph";
    pub const DISALLOWED_EXTRACT_TOOLS_ARG: &str = "--enable-extract-tools";
    pub const SMOKE_AGENT_NAME_FRAGMENT: &str = "smoke";
}

pub mod generated_paths {
    pub const AGENT_HANDOFFS_REFERENCE: &str =
        ".agents/skills/semantic-graph/references/agent-handoffs.md";
    pub const CODEX_AGENTS_DIR: &str = ".codex/agents/";
    pub const CONFIG_SNIPPET: &str = ".codex/config.semantic-graph.toml";
    pub const CSHARP_EXTRACTION_REFERENCE: &str =
        ".agents/skills/semantic-graph/references/csharp-extraction.md";
    pub const CSHARP_REFRESH_AGENT: &str = ".codex/agents/semantic-graph-csharp-refresh.toml";
    pub const EXPLORER_AGENT: &str = ".codex/agents/semantic-graph-explorer.toml";
    pub const LOCAL_TESTBEDS_REFERENCE: &str =
        ".agents/skills/semantic-graph/references/local-testbeds.md";
    pub const MCP_TOOLS_REFERENCE: &str = ".agents/skills/semantic-graph/references/mcp-tools.md";
    pub const PLAN_AUDITOR_AGENT: &str = ".codex/agents/semantic-graph-plan-auditor.toml";
    pub const README: &str = ".codex/semantic-graph/README.md";
    pub const ROUTE_VERIFIER_AGENT: &str = ".codex/agents/semantic-graph-route-verifier.toml";
    pub const RUST_EXTRACTION_REFERENCE: &str =
        ".agents/skills/semantic-graph/references/rust-extraction.md";
    pub const RUST_REFRESH_AGENT: &str = ".codex/agents/semantic-graph-rust-refresh.toml";
    pub const SKILL: &str = ".agents/skills/semantic-graph/SKILL.md";
    pub const SOUL_EXTRACTION_REFERENCE: &str =
        ".agents/skills/semantic-graph/references/soul-extraction.md";
    pub const TROUBLESHOOTING_REFERENCE: &str =
        ".agents/skills/semantic-graph/references/troubleshooting.md";

    pub const ALL: &[&str] = &[
        SKILL,
        AGENT_HANDOFFS_REFERENCE,
        CSHARP_EXTRACTION_REFERENCE,
        LOCAL_TESTBEDS_REFERENCE,
        MCP_TOOLS_REFERENCE,
        RUST_EXTRACTION_REFERENCE,
        SOUL_EXTRACTION_REFERENCE,
        TROUBLESHOOTING_REFERENCE,
        CSHARP_REFRESH_AGENT,
        EXPLORER_AGENT,
        PLAN_AUDITOR_AGENT,
        ROUTE_VERIFIER_AGENT,
        RUST_REFRESH_AGENT,
        CONFIG_SNIPPET,
        README,
    ];
}

pub mod generated_agent_names {
    pub const CSHARP_REFRESH: &str = "semantic-graph-csharp-refresh";
    pub const EXPLORER: &str = "semantic-graph-explorer";
    pub const PLAN_AUDITOR: &str = "semantic-graph-plan-auditor";
    pub const ROUTE_VERIFIER: &str = "semantic-graph-route-verifier";
    pub const RUST_REFRESH: &str = "semantic-graph-rust-refresh";

    pub const ALL: &[&str] = &[
        CSHARP_REFRESH,
        EXPLORER,
        PLAN_AUDITOR,
        ROUTE_VERIFIER,
        RUST_REFRESH,
    ];
}

pub mod mcp {
    pub const CONFIG_ROOT_TABLE: &str = "mcp_servers";
    pub const CONFIG_SNIPPET_ARTIFACT: &str = "config.semantic-graph";
    pub const MANAGED_SERVER_COMMAND: &str = ".refactor-radar/bin/semantic-graph-mcp-server";
    pub const RESOURCE_URI_PREFIX: &str = "semantic-graph://";
}

pub mod toml_fields {
    pub const ARGS: &str = "args";
    pub const COMMAND: &str = "command";
    pub const DESCRIPTION: &str = "description";
    pub const DEVELOPER_INSTRUCTIONS: &str = "developer_instructions";
    pub const ENABLED: &str = "enabled";
    pub const NAME: &str = "name";
    pub const REQUIRED: &str = "required";
}

pub mod render {
    pub const PROGRESSIVE_REFERENCES_HEADING: &str = "Progressive References";
    pub const REFERENCES_DIR: &str = "references";
    pub const README_TITLE: &str = "SemanticGraph Codex Assets";
    pub const SKILL_TITLE: &str = "SemanticGraph";
    pub const TEMP_RENDER_DIR_PREFIX: &str = "semantic-graph-agent-assets";
}

pub mod tests {
    pub const CHECK_CHANGED_TEMP_NAME: &str = "check-changed";
    pub const CHECK_DUPLICATE_TEMP_NAME: &str = "check-duplicate";
    pub const CHECK_MISSING_TEMP_NAME: &str = "check-missing";
    pub const CHECK_PASSES_TEMP_NAME: &str = "check-passes";
    pub const CHECK_PATH_ESCAPE_TEMP_NAME: &str = "check-path-escape";
    pub const CHECK_STALE_TEMP_NAME: &str = "check-stale";
    pub const FRAGMENTS_PATH: &str = "agent-assets/fragments";
    pub const GENERATED_EXPECTED_ROOT: &str = "agent-assets/expected/codex";
    pub const GENERATE_PATH_ESCAPE_TEMP_NAME: &str = "generate-path-escape";
    pub const GENERATE_WRITES_TEMP_NAME: &str = "generate-writes";
    pub const INVALID_TOML: &str = "not = [valid";
    pub const MANIFEST_TEST_PATH: &str = "manifest.toml";
    pub const MISSING_MCP_TOOLS_FRAGMENT: &str = "common/missing-mcp-tools.md";
    pub const OUTPUT_PATH_ESCAPE_LINE: &str = "output_path = \"../SKILL.md\"";
    pub const MCP_TOOL_REGISTRY_SOURCE: &str =
        "crates/semantic-graph-mcp-server/src/tools/tool_registry.rs";
    pub const RESOURCE_SOURCE_FILES: &[&str] = &[
        "crates/semantic-graph-mcp-server/src/resources/schema_resource.rs",
        "crates/semantic-graph-mcp-server/src/resources/workspace_resource.rs",
        "crates/semantic-graph-mcp-server/src/resources/routes_resource.rs",
        "crates/semantic-graph-mcp-server/src/resources/local_testbeds_resource.rs",
    ];
    pub const STALE_ARTIFACT: &str = ".codex/stale.toml";
    pub const TEMP_REPO_DIR_PREFIX: &str = "semantic-graph-agent-assets-test";
}
