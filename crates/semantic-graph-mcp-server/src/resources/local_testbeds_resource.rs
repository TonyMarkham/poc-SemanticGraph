pub const LOCAL_TESTBEDS_RESOURCE_URI: &str = "semantic-graph://local-testbeds";

pub fn local_testbeds_resource_text() -> String {
    [
        "Local SemanticGraph testbeds.",
        "",
        "apps/SemanticGraph.Visualizer is the browser-hosted Blazor WebAssembly graph UI testbed.",
        "crates/semantic-graph-visualizer-server is the local JSON-RPC backend used by that testbed.",
        "These projects are prior art only. They are not the durable MCP boundary and do not define MCP tool or resource names.",
    ]
    .join("\n")
}
