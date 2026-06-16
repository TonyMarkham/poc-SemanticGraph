mod server_adapter;
mod tool_output;

pub use server_adapter::serve_stdio;
pub(crate) use tool_output::{
    deserialize_tool_arguments, query_error_to_mcp, structured_tool_result,
};
