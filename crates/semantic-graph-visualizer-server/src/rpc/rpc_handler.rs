use crate::{
    VisualizerServerError, VisualizerServerResult,
    dto::{
        GraphEdgeDetailsParamsDto, GraphNodeDetailsParamsDto, GraphProjectionParamsDto,
        GraphSearchNodesParamsDto, JsonRpcErrorDto, JsonRpcRequestDto, JsonRpcResponseDto,
    },
    server::AppState,
};

use axum::{Json, body::Bytes, extract::State};
use serde_json::{Value, json};

const PARSE_ERROR: i64 = -32700;
const INVALID_REQUEST: i64 = -32600;
const METHOD_NOT_FOUND: i64 = -32601;
const INVALID_PARAMS: i64 = -32602;
const INTERNAL_ERROR: i64 = -32603;
const NOT_FOUND: i64 = -32004;

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
        "graph.node_details" => Json(handle_graph_node_details(state, request).await),
        "graph.edge_details" => Json(handle_graph_edge_details(state, request).await),
        "graph.search_nodes" => Json(handle_graph_search_nodes(state, request).await),
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
    let params = match deserialize_params::<GraphProjectionParamsDto>(request.params) {
        Ok(value) => value,
        Err(error) => {
            return error_from_server_error(id, error);
        }
    };

    let limit = match params.resolved_limit() {
        Ok(value) => value,
        Err(error) => {
            return error_from_server_error(id, error);
        }
    };

    result_response(id, state.query_service().projection(limit).await)
}

async fn handle_graph_node_details(
    state: AppState,
    request: JsonRpcRequestDto,
) -> JsonRpcResponseDto {
    let id = request.id.clone();
    let params = match deserialize_params::<GraphNodeDetailsParamsDto>(request.params) {
        Ok(value) => value,
        Err(error) => {
            return error_from_server_error(id, error);
        }
    };

    let node_id = match params.resolved_node_id() {
        Ok(value) => value,
        Err(error) => {
            return error_from_server_error(id, error);
        }
    };

    result_response(id, state.query_service().node_details(&node_id).await)
}

async fn handle_graph_edge_details(
    state: AppState,
    request: JsonRpcRequestDto,
) -> JsonRpcResponseDto {
    let id = request.id.clone();
    let params = match deserialize_params::<GraphEdgeDetailsParamsDto>(request.params) {
        Ok(value) => value,
        Err(error) => {
            return error_from_server_error(id, error);
        }
    };

    let edge_id = match params.resolved_edge_id() {
        Ok(value) => value,
        Err(error) => {
            return error_from_server_error(id, error);
        }
    };

    result_response(id, state.query_service().edge_details(&edge_id).await)
}

async fn handle_graph_search_nodes(
    state: AppState,
    request: JsonRpcRequestDto,
) -> JsonRpcResponseDto {
    let id = request.id.clone();
    let params = match deserialize_params::<GraphSearchNodesParamsDto>(request.params) {
        Ok(value) => value,
        Err(error) => {
            return error_from_server_error(id, error);
        }
    };

    let query = match params.resolved_query() {
        Ok(value) => value,
        Err(error) => {
            return error_from_server_error(id, error);
        }
    };

    let limit = match params.resolved_limit() {
        Ok(value) => value,
        Err(error) => {
            return error_from_server_error(id, error);
        }
    };

    result_response(id, state.query_service().search_nodes(&query, limit).await)
}

fn deserialize_params<T>(params: Option<Value>) -> VisualizerServerResult<T>
where
    T: serde::de::DeserializeOwned,
{
    let params_value = params.unwrap_or_else(|| json!({}));

    serde_json::from_value::<T>(params_value)
        .map_err(|source| VisualizerServerError::invalid_params(source.to_string()))
}

fn result_response<T>(id: Option<Value>, result: VisualizerServerResult<T>) -> JsonRpcResponseDto
where
    T: serde::Serialize,
{
    match result {
        Ok(value) => match serde_json::to_value(value) {
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
        VisualizerServerError::NotFound { .. } => NOT_FOUND,
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
