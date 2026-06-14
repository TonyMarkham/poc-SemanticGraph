use crate::{
    ServerConfig, VisualizerServerError, VisualizerServerResult, rpc::rpc_handler, server::AppState,
};

use axum::{Router, routing::post};
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;

pub async fn run_server(config: ServerConfig) -> VisualizerServerResult<()> {
    let state = AppState::new(config.database_path().to_path_buf());
    let app = Router::new()
        .route("/rpc", post(rpc_handler))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = TcpListener::bind(config.bind())
        .await
        .map_err(VisualizerServerError::io)?;

    println!(
        "semantic graph visualizer server listening on http://{} with database {}",
        config.bind(),
        config.database_path().display()
    );

    axum::serve(listener, app)
        .await
        .map_err(VisualizerServerError::io)
}
