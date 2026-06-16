use crate::{QueryError, QueryResult};

use serde_json::Value;

pub(crate) fn parse_json_value(raw_json: &str) -> QueryResult<Value> {
    serde_json::from_str(raw_json).map_err(QueryError::json)
}

pub(crate) fn parse_optional_json_value(raw_json: Option<String>) -> QueryResult<Option<Value>> {
    match raw_json {
        Some(value) => parse_json_value(&value).map(Some),
        None => Ok(None),
    }
}
