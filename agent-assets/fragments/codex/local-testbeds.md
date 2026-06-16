## Local Surfaces

`apps/SemanticGraph.Visualizer` is the browser-hosted Blazor WebAssembly graph UI testbed. `crates/semantic-graph-visualizer-server` is the local JSON-RPC backend used by that testbed.

These projects are optional prior art and smoke surfaces. They are not the durable MCP boundary, do not define generated Codex asset boundaries, and do not define MCP tool or resource names.

Use the visualizer only when the user asks for local graph UI behavior or when a visual smoke check is useful. Do not update the visualizer as part of Codex asset generation work unless the user explicitly asks for that separate change.
