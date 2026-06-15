use crate::{ConfigError, ConfigResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtractorMode {
    Serial,
    Threaded,
}

impl ExtractorMode {
    pub fn parse(setting: &str, value: &str) -> ConfigResult<Self> {
        match value {
            "serial" => Ok(Self::Serial),
            "threaded" => Ok(Self::Threaded),
            _ => Err(ConfigError::invalid_extractor_setting(
                setting,
                "must be either \"serial\" or \"threaded\"",
            )),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Serial => "serial",
            Self::Threaded => "threaded",
        }
    }
}
