use crate::error::QueryError;

use std::result::Result as StdResult;

pub type QueryResult<T> = StdResult<T, QueryError>;
