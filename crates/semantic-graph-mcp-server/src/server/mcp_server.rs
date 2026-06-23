use crate::{
    resources::ResourceRegistry,
    rmcp_integration::{deserialize_tool_arguments, query_error_to_mcp, structured_tool_result},
    server::ServerState,
    tools::{
        EdgeDetailsParams, EmptyToolParams, FTS_SEARCH, FileSummaryParams, FtsSearchParams,
        GRAPH_EDGE_DETAILS, GRAPH_FILE_SUMMARY, GRAPH_NEIGHBORS, GRAPH_NODE_DETAILS,
        GRAPH_PROJECTION, GRAPH_ROUTE_STATUS, GRAPH_SEARCH_NODES, GRAPH_SHORTEST_PATH, GRAPH_STATS,
        NeighborsParams, NodeDetailsParams, ProjectionParams, RouteStatusParams, SOUL_SEARCH,
        SearchNodesParams, ShortestPathParams, SoulSearchParams, ToolRegistry,
    },
};

use rmcp::{
    ErrorData, RoleServer, ServerHandler,
    model::{
        CallToolRequestParams, CallToolResult, Implementation, ListResourcesResult,
        ListToolsResult, PaginatedRequestParams, ProtocolVersion, ReadResourceRequestParams,
        ReadResourceResult, ServerCapabilities, ServerInfo,
    },
    service::RequestContext,
};
use serde_json::json;

#[derive(Debug, Clone)]
pub struct SemanticGraphMcpServer {
    state: ServerState,
}

impl SemanticGraphMcpServer {
    pub fn new(state: ServerState) -> Self {
        Self { state }
    }

    pub fn state(&self) -> &ServerState {
        &self.state
    }
}

impl ServerHandler for SemanticGraphMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_resources()
                .enable_tools()
                .build(),
        )
        .with_protocol_version(ProtocolVersion::V_2024_11_05)
        .with_server_info(
            Implementation::new(
                "semantic-graph-mcp-server",
                env!("CARGO_PKG_VERSION"),
            )
            .with_title("SemanticGraph MCP Server")
            .with_description("Read-only stdio MCP server for SemanticGraph SQLite stores"),
        )
        .with_instructions(
            "Use read-only graph tools and semantic-graph:// resources. This server does not run extractors or mutate the graph.",
        )
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult::with_all_items(ToolRegistry::tools()))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        match request.name.as_ref() {
            GRAPH_STATS => {
                let _params = deserialize_tool_arguments::<EmptyToolParams>(request.arguments)?;
                let stats = self
                    .state
                    .query_service()
                    .stats()
                    .await
                    .map_err(query_error_to_mcp)?;
                structured_tool_result("Graph statistics loaded.", stats)
            }
            GRAPH_SEARCH_NODES => {
                let params = deserialize_tool_arguments::<SearchNodesParams>(request.arguments)?;
                let results = self
                    .state
                    .query_service()
                    .search_nodes(params.into())
                    .await
                    .map_err(query_error_to_mcp)?;
                structured_tool_result("Node search completed.", results)
            }
            GRAPH_NODE_DETAILS => {
                let params = deserialize_tool_arguments::<NodeDetailsParams>(request.arguments)?;
                let details = self
                    .state
                    .query_service()
                    .node_details(params.into())
                    .await
                    .map_err(query_error_to_mcp)?;
                structured_tool_result("Node details loaded.", details)
            }
            GRAPH_EDGE_DETAILS => {
                let params = deserialize_tool_arguments::<EdgeDetailsParams>(request.arguments)?;
                let details = self
                    .state
                    .query_service()
                    .edge_details(params.into())
                    .await
                    .map_err(query_error_to_mcp)?;
                structured_tool_result("Edge details loaded.", details)
            }
            GRAPH_PROJECTION => {
                let params = deserialize_tool_arguments::<ProjectionParams>(request.arguments)?;
                let projection = self
                    .state
                    .query_service()
                    .projection(params.into())
                    .await
                    .map_err(query_error_to_mcp)?;
                structured_tool_result("Graph projection loaded.", projection)
            }
            GRAPH_NEIGHBORS => {
                let params = deserialize_tool_arguments::<NeighborsParams>(request.arguments)?;
                let neighbors = self
                    .state
                    .query_service()
                    .neighbors(params.into())
                    .await
                    .map_err(query_error_to_mcp)?;
                structured_tool_result("Node neighbors loaded.", neighbors)
            }
            GRAPH_SHORTEST_PATH => {
                let params = deserialize_tool_arguments::<ShortestPathParams>(request.arguments)?;
                let path = self
                    .state
                    .query_service()
                    .shortest_path(params.into())
                    .await
                    .map_err(query_error_to_mcp)?;
                structured_tool_result("Shortest path search completed.", path)
            }
            GRAPH_FILE_SUMMARY => {
                let params = deserialize_tool_arguments::<FileSummaryParams>(request.arguments)?;
                let summary = self
                    .state
                    .query_service()
                    .file_summary(params.into())
                    .await
                    .map_err(query_error_to_mcp)?;
                structured_tool_result("File summary loaded.", summary)
            }
            GRAPH_ROUTE_STATUS => {
                let params = deserialize_tool_arguments::<RouteStatusParams>(request.arguments)?;
                let results = self
                    .state
                    .query_service()
                    .route_status(params.into())
                    .await
                    .map_err(query_error_to_mcp)?;
                structured_tool_result("Route status loaded.", results)
            }
            FTS_SEARCH => {
                let params = deserialize_tool_arguments::<FtsSearchParams>(request.arguments)?;
                let service = self.state.fts_query_service().ok_or_else(|| {
                    ErrorData::invalid_params(
                        "FTS search is not configured for this MCP server",
                        None,
                    )
                })?;
                let results = service
                    .search(params.into())
                    .await
                    .map_err(query_error_to_mcp)?;
                structured_tool_result("FTS search completed.", results)
            }
            SOUL_SEARCH => {
                let params = deserialize_tool_arguments::<SoulSearchParams>(request.arguments)?;
                let results = self
                    .state
                    .query_service()
                    .soul_search(params.into())
                    .await
                    .map_err(query_error_to_mcp)?;
                structured_tool_result("Soul search completed.", results)
            }
            name => Err(ErrorData::invalid_params(
                "unknown tool",
                Some(json!({ "tool": name })),
            )),
        }
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        Ok(ListResourcesResult::with_all_items(
            ResourceRegistry::resources(),
        ))
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, ErrorData> {
        ResourceRegistry::read_resource(&self.state, &request.uri).await
    }
}
