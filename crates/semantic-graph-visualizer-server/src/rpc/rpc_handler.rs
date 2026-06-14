use crate::{
    VisualizerServerError,
    dto::{GraphProjectionParamsDto, JsonRpcErrorDto, JsonRpcRequestDto, JsonRpcResponseDto},
    server::AppState,
};

use axum::{Json, body::Bytes, extract::State};
use serde_json::{Value, json};

const PARSE_ERROR: i64 = -32700;
const INVALID_REQUEST: i64 = -32600;
const METHOD_NOT_FOUND: i64 = -32601;
const INVALID_PARAMS: i64 = -32602;
const INTERNAL_ERROR: i64 = -32603;

pub async fn rpc_handler(State(state): State<AppState>, body: Bytes) -> Json<JsonRpcResponseDto> {
    let request = match serde_json::from_slice::<JsonRpcRequestDto>(&body) {
        Ok(value) => value,
        Err(source) => {
            return Json(error_response(
                None,
                PARSE_ERROR,
                "parse error",
                Some(json!({ "detail": source.to_string() })),
            ));
        }
    };

    let id = request.id.clone();

    if request.jsonrpc != "2.0" {
        return Json(error_response(
            id,
            INVALID_REQUEST,
            "invalid request",
            Some(json!({ "detail": "jsonrpc must be 2.0" })),
        ));
    }

    match request.method.as_str() {
        "graph.projection" => Json(handle_graph_projection(state, request).await),
        _ => Json(error_response(
            id,
            METHOD_NOT_FOUND,
            "method not found",
            Some(json!({ "method": request.method })),
        )),
    }
}

async fn handle_graph_projection(
    state: AppState,
    request: JsonRpcRequestDto,
) -> JsonRpcResponseDto {
    let id = request.id.clone();
    let params_value = request.params.unwrap_or_else(|| json!({}));
    let params = match serde_json::from_value::<GraphProjectionParamsDto>(params_value) {
        Ok(value) => value,
        Err(source) => {
            return error_response(
                id,
                INVALID_PARAMS,
                "invalid params",
                Some(json!({ "detail": source.to_string() })),
            );
        }
    };

    let limit = match params.resolved_limit() {
        Ok(value) => value,
        Err(error) => {
            return error_from_server_error(id, error);
        }
    };

    match state.projection_service().projection(limit).await {
        Ok(projection) => match serde_json::to_value(projection) {
            Ok(value) => JsonRpcResponseDto::result(id, value),
            Err(source) => error_from_server_error(id, VisualizerServerError::json(source)),
        },
        Err(error) => error_from_server_error(id, error),
    }
}

fn error_from_server_error(id: Option<Value>, error: VisualizerServerError) -> JsonRpcResponseDto {
    let code = match error {
        VisualizerServerError::InvalidParams { .. } => INVALID_PARAMS,
        VisualizerServerError::InvalidRequest { .. } => INVALID_REQUEST,
        VisualizerServerError::Database { .. }
        | VisualizerServerError::Io { .. }
        | VisualizerServerError::InvalidConfig { .. }
        | VisualizerServerError::Json { .. } => INTERNAL_ERROR,
    };

    error_response(
        id,
        code,
        error.message(),
        Some(json!({ "detail": error.to_string() })),
    )
}

fn error_response(
    id: Option<Value>,
    code: i64,
    message: &str,
    data: Option<Value>,
) -> JsonRpcResponseDto {
    JsonRpcResponseDto::error(
        id,
        JsonRpcErrorDto {
            code,
            message: message.to_string(),
            data,
        },
    )
}
