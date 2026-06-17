//! Internal owned command payloads.
//!
//! These DTOs intentionally parallel `crate::input`: the public input structs
//! borrow from caller-owned data, while these models own strings and JSON so a
//! command can be queued and processed after the caller's stack frame is gone.

mod owned_close_stale_file_input;
mod owned_close_stale_fts_documents_input;
mod owned_close_stale_route_input;
mod owned_edge_evidence_input;
mod owned_edge_input;
mod owned_file_input;
mod owned_fts_document_input;
mod owned_node_input;
mod owned_occurrence_input;
mod owned_route_observation_input;
mod owned_route_status_complete_input;
mod owned_route_status_fail_input;
mod owned_route_status_start_input;

pub(crate) use owned_close_stale_file_input::OwnedCloseStaleFileInput;
pub(crate) use owned_close_stale_fts_documents_input::OwnedCloseStaleFtsDocumentsInput;
pub(crate) use owned_close_stale_route_input::OwnedCloseStaleRouteInput;
pub(crate) use owned_edge_evidence_input::OwnedEdgeEvidenceInput;
pub(crate) use owned_edge_input::OwnedEdgeInput;
pub(crate) use owned_file_input::OwnedFileInput;
pub(crate) use owned_fts_document_input::OwnedFtsDocumentInput;
pub(crate) use owned_node_input::OwnedNodeInput;
pub(crate) use owned_occurrence_input::OwnedOccurrenceInput;
pub(crate) use owned_route_observation_input::OwnedRouteObservationInput;
pub(crate) use owned_route_status_complete_input::OwnedRouteStatusCompleteInput;
pub(crate) use owned_route_status_fail_input::OwnedRouteStatusFailInput;
pub(crate) use owned_route_status_start_input::OwnedRouteStatusStartInput;
