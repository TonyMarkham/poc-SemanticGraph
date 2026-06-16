#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueryServiceConfigValues {
    pub latest_run_limit: i64,
    pub max_search_limit: i64,
    pub max_projection_limit: i64,
    pub max_neighbors_limit: i64,
    pub max_file_edge_limit: i64,
    pub max_route_status_limit: i64,
    pub max_shortest_path_depth: i64,
    pub max_shortest_path_visited: i64,
}
