# Troubleshooting

## Missing Database

If `graph_stats` cannot open the database, check the MCP server args, the SemanticGraph config path, and whether the expected SQLite file exists. Use an explicit `--database-path` only for temporary databases or manual launches that should bypass config discovery.

## Stale Routes

If graph facts look old, inspect `graph_route_status` for the relevant workspace, file, and route. Treat missing or failed routes as insufficient evidence for current behavior. Refresh only when the user asked for implementation, refresh, or validation work that requires current graph facts.

## Unavailable MCP Server

If the MCP server is not configured, use the generated `.codex/config.semantic-graph.toml` snippet as the project-local config source for a later structural merge. Do not write global Codex configuration as part of this skill.

## Evidence Gaps

When graph evidence is absent, contradictory, or stale, say so. Separate confirmed graph facts from source-code inferences and unknowns.
