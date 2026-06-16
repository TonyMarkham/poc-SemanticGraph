use crate::sanitize::{DEFAULT_TEXT_CAP, sanitize_transcript_text};

use rmcp::{ErrorData, model::CallToolResult};
use semantic_graph_query::QueryError;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

pub(crate) fn structured_tool_result(
    summary: impl AsRef<str>,
    data: impl Serialize,
) -> Result<CallToolResult, ErrorData> {
    let data = serde_json::to_value(data).map_err(|error| {
        ErrorData::internal_error(
            "failed to serialize tool result",
            Some(
                json!({ "error": sanitize_transcript_text(&error.to_string(), DEFAULT_TEXT_CAP) }),
            ),
        )
    })?;

    Ok(CallToolResult::structured(json!({
        "summary": sanitize_transcript_text(summary.as_ref(), DEFAULT_TEXT_CAP),
        "data": data,
    })))
}

pub(crate) fn deserialize_tool_arguments<T>(
    arguments: Option<rmcp::model::JsonObject>,
) -> Result<T, ErrorData>
where
    T: DeserializeOwned,
{
    serde_json::from_value(Value::Object(arguments.unwrap_or_default())).map_err(|error| {
        ErrorData::invalid_params(
            "invalid tool arguments",
            Some(
                json!({ "error": sanitize_transcript_text(&error.to_string(), DEFAULT_TEXT_CAP) }),
            ),
        )
    })
}

pub(crate) fn query_error_to_mcp(error: QueryError) -> ErrorData {
    match error {
        QueryError::InvalidParams { message, .. } => {
            ErrorData::invalid_params(sanitize_transcript_text(&message, DEFAULT_TEXT_CAP), None)
        }
        QueryError::NotFound { message, .. } => ErrorData::resource_not_found(
            sanitize_transcript_text(&message, DEFAULT_TEXT_CAP),
            None,
        ),
        QueryError::Database { .. } => ErrorData::internal_error("database error", None),
        QueryError::Json { .. } => ErrorData::internal_error("json error", None),
    }
}

#[cfg(test)]
mod tests {
    use crate::rmcp_integration::query_error_to_mcp;

    use rmcp::model::ErrorCode;
    use semantic_graph_query::QueryError;

    #[test]
    fn maps_invalid_params_to_mcp_invalid_params() {
        let error = query_error_to_mcp(QueryError::invalid_params("bad input"));

        assert_eq!(ErrorCode::INVALID_PARAMS, error.code);
        assert_eq!("bad input", error.message);
    }

    #[test]
    fn maps_database_errors_to_sanitized_internal_error() {
        let error = query_error_to_mcp(QueryError::database(sqlx_error()));

        assert_eq!(ErrorCode::INTERNAL_ERROR, error.code);
        assert_eq!("database error", error.message);
    }

    fn sqlx_error() -> sqlx::Error {
        sqlx::Error::RowNotFound
    }
}
