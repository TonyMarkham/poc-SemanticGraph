//! Public borrowed write inputs.
//!
//! These DTOs keep caller code ergonomic: extractors and tests can pass string
//! slices and optional JSON values without first allocating DB-manager-owned
//! command payloads. `crate::models` intentionally mirrors this module with
//! owned versions that can safely cross the async write queue boundary.

mod close_stale_file_input;
mod close_stale_route_input;
mod edge_evidence_input;
mod edge_input;
mod file_input;
mod node_input;
mod occurrence_input;
mod route_observation_input;
mod route_status_complete_input;
mod route_status_fail_input;
mod route_status_start_input;
mod text_range;

// ---------------------------------------------------------------------------------------------- //

pub use close_stale_file_input::CloseStaleFileInput;
pub use close_stale_route_input::CloseStaleRouteInput;
pub use edge_evidence_input::EdgeEvidenceInput;
pub use edge_input::EdgeInput;
pub use file_input::FileInput;
pub use node_input::NodeInput;
pub use occurrence_input::OccurrenceInput;
pub use route_observation_input::RouteObservationInput;
pub use route_status_complete_input::RouteStatusCompleteInput;
pub use route_status_fail_input::RouteStatusFailInput;
pub use route_status_start_input::RouteStatusStartInput;
pub use text_range::TextRange;
