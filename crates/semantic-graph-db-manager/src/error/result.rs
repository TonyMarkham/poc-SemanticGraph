use crate::DbManagerError;

use std::result::Result as StdResult;

pub type DbManagerResult<T> = StdResult<T, DbManagerError>;
