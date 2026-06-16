pub mod cli {
    pub const CODEX_COMMAND_NAME: &str = "codex";
    pub const COMMAND_NAME: &str = "semantic-graph";
    pub const INSTALL_COMMAND_NAME: &str = "install";
    pub const UNINSTALL_COMMAND_NAME: &str = "uninstall";
}

pub mod codex_paths {
    pub const CONFIG: &str = ".codex/config.toml";
    pub const INSTALL_MANIFEST: &str = ".codex/semantic-graph/install-manifest.json";
}

pub mod manifest {
    pub const ASSET_GENERATION: &str = "semantic-graph-agent-assets";
    pub const INSTALLER_CRATE: &str = "semantic-graph-cli";
    pub const SCHEMA_VERSION: u32 = 1;
}
