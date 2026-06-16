use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct McpServerToml<'a> {
    pub(crate) command: &'a str,
    pub(crate) args: &'a [String],
    pub(crate) enabled: bool,
    pub(crate) required: bool,
    pub(crate) startup_timeout_sec: u64,
    pub(crate) tool_timeout_sec: u64,
}
