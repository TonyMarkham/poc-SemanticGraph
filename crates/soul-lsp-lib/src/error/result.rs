use crate::SoulLspLibError;

use std::result::Result as StdResult;

pub type SoulLspLibResult<T> = StdResult<T, SoulLspLibError>;
