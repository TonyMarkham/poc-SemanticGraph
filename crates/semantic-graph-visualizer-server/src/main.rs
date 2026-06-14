use semantic_graph_visualizer_server::{ServerConfig, VisualizerServerResult, run_server};

#[tokio::main]
async fn main() -> VisualizerServerResult<()> {
    let config = ServerConfig::from_env_and_args()?;
    run_server(config).await
}
