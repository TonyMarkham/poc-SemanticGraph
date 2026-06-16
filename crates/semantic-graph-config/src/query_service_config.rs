use crate::{ConfigError, ConfigResult, QueryServiceConfigValues, RawQueryServiceConfig};

const DEFAULT_LATEST_RUN_LIMIT: i64 = 10;
const DEFAULT_MAX_SEARCH_LIMIT: i64 = 50;
const DEFAULT_MAX_PROJECTION_LIMIT: i64 = 1_000;
const DEFAULT_MAX_NEIGHBORS_LIMIT: i64 = 100;
const DEFAULT_MAX_FILE_EDGE_LIMIT: i64 = 200;
const DEFAULT_MAX_ROUTE_STATUS_LIMIT: i64 = 200;
const DEFAULT_MAX_SHORTEST_PATH_DEPTH: i64 = 12;
const DEFAULT_MAX_SHORTEST_PATH_VISITED: i64 = 5_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryServiceConfig {
    latest_run_limit: i64,
    max_search_limit: i64,
    max_projection_limit: i64,
    max_neighbors_limit: i64,
    max_file_edge_limit: i64,
    max_route_status_limit: i64,
    max_shortest_path_depth: i64,
    max_shortest_path_visited: i64,
}

impl QueryServiceConfig {
    pub fn new(values: QueryServiceConfigValues) -> ConfigResult<Self> {
        validate_positive_i64("query-service.latest_run_limit", values.latest_run_limit)?;
        validate_positive_i64("query-service.max_search_limit", values.max_search_limit)?;
        validate_positive_i64(
            "query-service.max_projection_limit",
            values.max_projection_limit,
        )?;
        validate_positive_i64(
            "query-service.max_neighbors_limit",
            values.max_neighbors_limit,
        )?;
        validate_positive_i64(
            "query-service.max_file_edge_limit",
            values.max_file_edge_limit,
        )?;
        validate_positive_i64(
            "query-service.max_route_status_limit",
            values.max_route_status_limit,
        )?;
        validate_positive_i64(
            "query-service.max_shortest_path_depth",
            values.max_shortest_path_depth,
        )?;
        validate_positive_i64(
            "query-service.max_shortest_path_visited",
            values.max_shortest_path_visited,
        )?;

        Ok(Self {
            latest_run_limit: values.latest_run_limit,
            max_search_limit: values.max_search_limit,
            max_projection_limit: values.max_projection_limit,
            max_neighbors_limit: values.max_neighbors_limit,
            max_file_edge_limit: values.max_file_edge_limit,
            max_route_status_limit: values.max_route_status_limit,
            max_shortest_path_depth: values.max_shortest_path_depth,
            max_shortest_path_visited: values.max_shortest_path_visited,
        })
    }

    pub(crate) fn from_raw(raw: Option<RawQueryServiceConfig>) -> ConfigResult<Self> {
        let Some(raw) = raw else {
            return Ok(Self::default());
        };

        Self::new(QueryServiceConfigValues {
            latest_run_limit: raw.latest_run_limit.unwrap_or(DEFAULT_LATEST_RUN_LIMIT),
            max_search_limit: raw.max_search_limit.unwrap_or(DEFAULT_MAX_SEARCH_LIMIT),
            max_projection_limit: raw
                .max_projection_limit
                .unwrap_or(DEFAULT_MAX_PROJECTION_LIMIT),
            max_neighbors_limit: raw
                .max_neighbors_limit
                .unwrap_or(DEFAULT_MAX_NEIGHBORS_LIMIT),
            max_file_edge_limit: raw
                .max_file_edge_limit
                .unwrap_or(DEFAULT_MAX_FILE_EDGE_LIMIT),
            max_route_status_limit: raw
                .max_route_status_limit
                .unwrap_or(DEFAULT_MAX_ROUTE_STATUS_LIMIT),
            max_shortest_path_depth: raw
                .max_shortest_path_depth
                .unwrap_or(DEFAULT_MAX_SHORTEST_PATH_DEPTH),
            max_shortest_path_visited: raw
                .max_shortest_path_visited
                .unwrap_or(DEFAULT_MAX_SHORTEST_PATH_VISITED),
        })
    }

    pub fn latest_run_limit(&self) -> i64 {
        self.latest_run_limit
    }

    pub fn max_search_limit(&self) -> i64 {
        self.max_search_limit
    }

    pub fn max_projection_limit(&self) -> i64 {
        self.max_projection_limit
    }

    pub fn max_neighbors_limit(&self) -> i64 {
        self.max_neighbors_limit
    }

    pub fn max_file_edge_limit(&self) -> i64 {
        self.max_file_edge_limit
    }

    pub fn max_route_status_limit(&self) -> i64 {
        self.max_route_status_limit
    }

    pub fn max_shortest_path_depth(&self) -> i64 {
        self.max_shortest_path_depth
    }

    pub fn max_shortest_path_visited(&self) -> i64 {
        self.max_shortest_path_visited
    }
}

impl Default for QueryServiceConfig {
    fn default() -> Self {
        Self {
            latest_run_limit: DEFAULT_LATEST_RUN_LIMIT,
            max_search_limit: DEFAULT_MAX_SEARCH_LIMIT,
            max_projection_limit: DEFAULT_MAX_PROJECTION_LIMIT,
            max_neighbors_limit: DEFAULT_MAX_NEIGHBORS_LIMIT,
            max_file_edge_limit: DEFAULT_MAX_FILE_EDGE_LIMIT,
            max_route_status_limit: DEFAULT_MAX_ROUTE_STATUS_LIMIT,
            max_shortest_path_depth: DEFAULT_MAX_SHORTEST_PATH_DEPTH,
            max_shortest_path_visited: DEFAULT_MAX_SHORTEST_PATH_VISITED,
        }
    }
}

fn validate_positive_i64(setting: &str, value: i64) -> ConfigResult<()> {
    if value <= 0 {
        return Err(ConfigError::invalid_query_service_setting(
            setting,
            "must be greater than zero",
        ));
    }

    Ok(())
}
