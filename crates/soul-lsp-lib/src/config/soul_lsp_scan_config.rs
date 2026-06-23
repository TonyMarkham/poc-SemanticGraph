#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoulLspScanConfig {
    excluded_dirs: Vec<String>,
    excluded_dir_suffixes: Vec<String>,
    excluded_bin_except_under: Vec<String>,
}

impl SoulLspScanConfig {
    pub fn new(
        excluded_dirs: Vec<String>,
        excluded_dir_suffixes: Vec<String>,
        excluded_bin_except_under: Vec<String>,
    ) -> Self {
        Self {
            excluded_dirs,
            excluded_dir_suffixes,
            excluded_bin_except_under,
        }
    }

    pub fn excluded_dirs(&self) -> &[String] {
        &self.excluded_dirs
    }

    pub fn excluded_dir_suffixes(&self) -> &[String] {
        &self.excluded_dir_suffixes
    }

    pub fn excluded_bin_except_under(&self) -> &[String] {
        &self.excluded_bin_except_under
    }
}

impl Default for SoulLspScanConfig {
    fn default() -> Self {
        Self {
            excluded_dirs: [
                ".git",
                ".soul",
                "target",
                ".idea",
                ".vscode",
                ".vs",
                ".codex",
                "node_modules",
                "obj",
            ]
            .into_iter()
            .map(ToString::to_string)
            .collect(),
            excluded_dir_suffixes: ["Tests", ".Tests", "tests", ".tests"]
                .into_iter()
                .map(ToString::to_string)
                .collect(),
            excluded_bin_except_under: ["src"].into_iter().map(ToString::to_string).collect(),
        }
    }
}
