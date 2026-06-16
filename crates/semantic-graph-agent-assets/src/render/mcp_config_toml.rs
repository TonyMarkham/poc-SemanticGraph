use crate::render::McpServerToml;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Serialize)]
pub(crate) struct McpConfigToml<'a> {
    pub(crate) mcp_servers: BTreeMap<&'a str, McpServerToml<'a>>,
}
