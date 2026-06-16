use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, ValueEnum)]
pub enum McpInstallMode {
    #[serde(rename = "read-only")]
    #[value(name = "read-only")]
    ReadOnly,

    #[serde(rename = "disabled")]
    #[value(name = "disabled")]
    Disabled,
}

impl McpInstallMode {
    pub fn enabled(self) -> bool {
        match self {
            Self::ReadOnly => true,
            Self::Disabled => false,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::ReadOnly => "read-only",
            Self::Disabled => "disabled",
        }
    }
}

impl std::fmt::Display for McpInstallMode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}
