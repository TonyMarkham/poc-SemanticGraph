use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct RawQueryServiceConfig {
    pub(crate) latest_run_limit: Option<i64>,
    pub(crate) max_search_limit: Option<i64>,
    pub(crate) max_projection_limit: Option<i64>,
    pub(crate) max_neighbors_limit: Option<i64>,
    pub(crate) max_file_edge_limit: Option<i64>,
    pub(crate) max_route_status_limit: Option<i64>,
    pub(crate) max_shortest_path_depth: Option<i64>,
    pub(crate) max_shortest_path_visited: Option<i64>,
}
