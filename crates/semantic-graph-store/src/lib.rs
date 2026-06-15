mod error;
mod store;
#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------------------------- //

pub use error::{GraphStoreError, GraphStoreResult};
pub use semantic_graph_db_manager::{TextRange, edge_id, node_id};
pub use store::{GraphStore, GraphStoreStats};
