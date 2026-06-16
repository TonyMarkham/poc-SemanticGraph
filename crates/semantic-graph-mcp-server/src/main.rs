use semantic_graph_mcp_server::run_from_env;

#[tokio::main]
async fn main() {
    if let Err(error) = run_from_env().await {
        eprintln!(
            "semantic-graph-mcp-server setup failed: {}",
            error.user_message()
        );
        std::process::exit(1);
    }
}
