use crate::model::RouteStatus;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RouteStatusResults {
    pub statuses: Vec<RouteStatus>,
    pub requested_limit: Option<i64>,
    pub applied_limit: i64,
}
