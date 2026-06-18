use crate::tools::{
    EdgeDetailsParams, EmptyToolParams, FileSummaryParams, FtsSearchParams, NeighborsParams,
    NodeDetailsParams, ProjectionParams, RouteStatusParams, SearchNodesParams, ShortestPathParams,
};

use rmcp::{
    handler::server::tool::schema_for_type,
    model::{Tool, ToolAnnotations},
};

pub const GRAPH_STATS: &str = "graph_stats";
pub const GRAPH_SEARCH_NODES: &str = "graph_search_nodes";
pub const GRAPH_NODE_DETAILS: &str = "graph_node_details";
pub const GRAPH_EDGE_DETAILS: &str = "graph_edge_details";
pub const GRAPH_PROJECTION: &str = "graph_projection";
pub const GRAPH_NEIGHBORS: &str = "graph_neighbors";
pub const GRAPH_SHORTEST_PATH: &str = "graph_shortest_path";
pub const GRAPH_FILE_SUMMARY: &str = "graph_file_summary";
pub const GRAPH_ROUTE_STATUS: &str = "graph_route_status";
pub const FTS_SEARCH: &str = "fts_search";

pub struct ToolRegistry;

impl ToolRegistry {
    #[cfg(test)]
    pub fn tool_names() -> Vec<&'static str> {
        vec![
            GRAPH_STATS,
            GRAPH_SEARCH_NODES,
            GRAPH_NODE_DETAILS,
            GRAPH_EDGE_DETAILS,
            GRAPH_PROJECTION,
            GRAPH_NEIGHBORS,
            GRAPH_SHORTEST_PATH,
            GRAPH_FILE_SUMMARY,
            GRAPH_ROUTE_STATUS,
            FTS_SEARCH,
        ]
    }

    pub fn tools() -> Vec<Tool> {
        vec![
            tool::<EmptyToolParams>(
                GRAPH_STATS,
                "Return database-wide graph counts and latest extraction runs.",
            ),
            tool::<SearchNodesParams>(
                GRAPH_SEARCH_NODES,
                "Search active nodes by label, qualified name, or source path.",
            ),
            tool::<NodeDetailsParams>(
                GRAPH_NODE_DETAILS,
                "Return one node with relation summaries and occurrences.",
            ),
            tool::<EdgeDetailsParams>(
                GRAPH_EDGE_DETAILS,
                "Return one edge with endpoints and evidence.",
            ),
            tool::<ProjectionParams>(
                GRAPH_PROJECTION,
                "Return a bounded active graph projection.",
            ),
            tool::<NeighborsParams>(
                GRAPH_NEIGHBORS,
                "Return bounded incoming and outgoing active neighbors for a node.",
            ),
            tool::<ShortestPathParams>(
                GRAPH_SHORTEST_PATH,
                "Return a bounded active path between two nodes.",
            ),
            tool::<FileSummaryParams>(
                GRAPH_FILE_SUMMARY,
                "Return symbols, touching edges, and route freshness for a known database file path.",
            ),
            tool::<RouteStatusParams>(
                GRAPH_ROUTE_STATUS,
                "Return route freshness rows filtered by workspace, route, scope, or file.",
            ),
            tool::<FtsSearchParams>(
                FTS_SEARCH,
                "Search indexed file contents with Tantivy-backed ranking and SQLite snippets.",
            ),
        ]
    }
}

fn tool<T>(name: &'static str, description: &'static str) -> Tool
where
    T: schemars::JsonSchema + 'static,
{
    Tool::new(name, description, schema_for_type::<T>()).with_annotations(
        ToolAnnotations::new()
            .read_only(true)
            .destructive(false)
            .open_world(false),
    )
}

#[cfg(test)]
mod tests {
    use crate::tools::{
        FTS_SEARCH, GRAPH_EDGE_DETAILS, GRAPH_FILE_SUMMARY, GRAPH_NEIGHBORS, GRAPH_NODE_DETAILS,
        GRAPH_PROJECTION, GRAPH_ROUTE_STATUS, GRAPH_SEARCH_NODES, GRAPH_SHORTEST_PATH, GRAPH_STATS,
        NodeDetailsParams, ToolRegistry,
    };

    #[test]
    fn lists_every_phase_two_tool_once() {
        assert_eq!(
            vec![
                GRAPH_STATS,
                GRAPH_SEARCH_NODES,
                GRAPH_NODE_DETAILS,
                GRAPH_EDGE_DETAILS,
                GRAPH_PROJECTION,
                GRAPH_NEIGHBORS,
                GRAPH_SHORTEST_PATH,
                GRAPH_FILE_SUMMARY,
                GRAPH_ROUTE_STATUS,
                FTS_SEARCH,
            ],
            ToolRegistry::tool_names()
        );

        let tools = ToolRegistry::tools();
        assert_eq!(ToolRegistry::tool_names().len(), tools.len());
    }

    #[test]
    fn required_tool_arguments_are_required() -> Result<(), Box<dyn std::error::Error>> {
        let error = serde_json::from_value::<NodeDetailsParams>(serde_json::json!({}))
            .err()
            .ok_or("missing nodeId should fail")?;

        assert!(error.to_string().contains("missing field"));
        Ok(())
    }
}
