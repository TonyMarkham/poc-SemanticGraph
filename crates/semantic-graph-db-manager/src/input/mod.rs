//! Public write inputs.
//!
//! Most DTOs keep caller code ergonomic: extractors and tests can pass string
//! slices and optional JSON values without first allocating DB-manager-owned
//! command payloads. `crate::models` intentionally mirrors those borrowed DTOs
//! with owned versions that can safely cross the async write queue boundary.

mod close_stale_file_input;
mod close_stale_fts_documents_input;
mod close_stale_route_input;
mod document_symbol_write_batch_close_stale_route_input;
mod document_symbol_write_batch_edge_evidence_input;
mod document_symbol_write_batch_file_input;
mod document_symbol_write_batch_input;
mod document_symbol_write_batch_node_input;
mod document_symbol_write_batch_observation_input;
mod document_symbol_write_batch_occurrence_input;
mod document_symbol_write_batch_route_status_complete_input;
mod document_symbol_write_batch_route_status_start_input;
mod document_symbol_write_batch_summary;
mod edge_evidence_input;
mod edge_input;
mod file_input;
mod fts_write_batch_document_input;
mod fts_write_batch_input;
mod fts_write_batch_seen_document_input;
mod node_input;
mod occurrence_input;
mod route_observation_input;
mod route_status_complete_input;
mod route_status_fail_input;
mod route_status_start_input;
mod route_write_batch_edge_evidence_input;
mod route_write_batch_edge_input;
mod route_write_batch_input;
mod route_write_batch_observation_input;
mod route_write_batch_occurrence_input;
mod text_range;

// ---------------------------------------------------------------------------------------------- //

pub use close_stale_file_input::CloseStaleFileInput;
pub use close_stale_fts_documents_input::CloseStaleFtsDocumentsInput;
pub use close_stale_route_input::CloseStaleRouteInput;
pub use document_symbol_write_batch_close_stale_route_input::DocumentSymbolWriteBatchCloseStaleRouteInput;
pub use document_symbol_write_batch_edge_evidence_input::DocumentSymbolWriteBatchEdgeEvidenceInput;
pub use document_symbol_write_batch_file_input::DocumentSymbolWriteBatchFileInput;
pub use document_symbol_write_batch_input::DocumentSymbolWriteBatchInput;
pub use document_symbol_write_batch_node_input::DocumentSymbolWriteBatchNodeInput;
pub use document_symbol_write_batch_observation_input::DocumentSymbolWriteBatchObservationInput;
pub use document_symbol_write_batch_occurrence_input::DocumentSymbolWriteBatchOccurrenceInput;
pub use document_symbol_write_batch_route_status_complete_input::DocumentSymbolWriteBatchRouteStatusCompleteInput;
pub use document_symbol_write_batch_route_status_start_input::DocumentSymbolWriteBatchRouteStatusStartInput;
pub use document_symbol_write_batch_summary::DocumentSymbolWriteBatchSummary;
pub use edge_evidence_input::EdgeEvidenceInput;
pub use edge_input::EdgeInput;
pub use file_input::FileInput;
pub use fts_write_batch_document_input::FtsWriteBatchDocumentInput;
pub use fts_write_batch_input::FtsWriteBatchInput;
pub use fts_write_batch_seen_document_input::FtsWriteBatchSeenDocumentInput;
pub use node_input::NodeInput;
pub use occurrence_input::OccurrenceInput;
pub use route_observation_input::RouteObservationInput;
pub use route_status_complete_input::RouteStatusCompleteInput;
pub use route_status_fail_input::RouteStatusFailInput;
pub use route_status_start_input::RouteStatusStartInput;
pub use route_write_batch_edge_evidence_input::RouteWriteBatchEdgeEvidenceInput;
pub use route_write_batch_edge_input::RouteWriteBatchEdgeInput;
pub use route_write_batch_input::RouteWriteBatchInput;
pub use route_write_batch_observation_input::RouteWriteBatchObservationInput;
pub use route_write_batch_occurrence_input::RouteWriteBatchOccurrenceInput;
pub use text_range::TextRange;
