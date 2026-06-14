use semantic_graph_visualizer_server::{ServerConfig, VisualizerServerError, run_server};

#[tokio::main]
async fn main() -> Result<(), VisualizerServerError> {
    let config = ServerConfig::from_env_and_args()?;
    run_server(config).await
}
