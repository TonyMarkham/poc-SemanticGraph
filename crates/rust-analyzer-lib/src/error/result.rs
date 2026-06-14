use crate::RustAnalyzerLibError;

use std::result::Result as StdResult;

pub type RustAnalyzerLibResult<T> = StdResult<T, RustAnalyzerLibError>;
