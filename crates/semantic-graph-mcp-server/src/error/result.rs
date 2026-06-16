use crate::error::McpServerError;

pub type McpServerResult<T> = Result<T, McpServerError>;
