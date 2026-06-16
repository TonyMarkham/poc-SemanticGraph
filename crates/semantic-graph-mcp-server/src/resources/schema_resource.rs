pub const SCHEMA_RESOURCE_URI: &str = "semantic-graph://schema";

pub fn schema_resource_text() -> String {
    [
        "SemanticGraph SQLite schema summary.",
        "",
        "workspaces: workspace roots and language labels.",
        "extraction_runs: extraction run provenance, status, and timing.",
        "files: source files known to a workspace.",
        "nodes: canonical graph nodes with kind, symbol identity, ranges, and stale state.",
        "edges: directed canonical graph edges with relation, confidence, weight, and stale state.",
        "occurrences: source proof rows for definitions, references, and calls.",
        "edge_evidence: source proof rows for canonical edges.",
        "extraction_route_status: freshness state for extractor routes by scope.",
        "route_observations: per-run observations attached to route freshness.",
    ]
    .join("\n")
}
