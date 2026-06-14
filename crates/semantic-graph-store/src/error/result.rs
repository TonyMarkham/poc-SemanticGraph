use crate::GraphStoreError;

use std::result::Result as StdResult;

pub type GraphStoreResult<T> = StdResult<T, GraphStoreError>;
