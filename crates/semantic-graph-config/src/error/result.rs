use crate::error::ConfigError;
use std::result::Result as StdResult;

pub type ConfigResult<T> = StdResult<T, ConfigError>;
