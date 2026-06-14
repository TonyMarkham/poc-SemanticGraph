mod error;
mod ids;
mod store;
#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------------------------- //

pub use error::{GraphStoreError, GraphStoreResult};
pub use ids::{edge_id, node_id};
pub use store::{
    DemoSeedSummary, EdgeEvidenceInput, EdgeInput, FileInput, GraphStore, GraphStoreStats,
    NodeInput, OccurrenceInput, TextRange,
};
