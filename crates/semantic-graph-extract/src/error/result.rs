use crate::ExtractError;

use std::result::Result as StdResult;

pub type ExtractResult<T> = StdResult<T, ExtractError>;
