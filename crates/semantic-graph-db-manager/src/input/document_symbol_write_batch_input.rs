use crate::{
    DocumentSymbolWriteBatchCloseStaleRouteInput, DocumentSymbolWriteBatchEdgeEvidenceInput,
    DocumentSymbolWriteBatchFileInput, DocumentSymbolWriteBatchNodeInput,
    DocumentSymbolWriteBatchObservationInput, DocumentSymbolWriteBatchOccurrenceInput,
    DocumentSymbolWriteBatchRouteStatusCompleteInput,
    DocumentSymbolWriteBatchRouteStatusStartInput, RouteWriteBatchEdgeInput,
};

#[derive(Debug, Clone, Default)]
pub struct DocumentSymbolWriteBatchInput {
    pub files: Vec<DocumentSymbolWriteBatchFileInput>,
    pub route_status_starts: Vec<DocumentSymbolWriteBatchRouteStatusStartInput>,
    pub nodes: Vec<DocumentSymbolWriteBatchNodeInput>,
    pub occurrences: Vec<DocumentSymbolWriteBatchOccurrenceInput>,
    pub edges: Vec<RouteWriteBatchEdgeInput>,
    pub edge_evidence: Vec<DocumentSymbolWriteBatchEdgeEvidenceInput>,
    pub route_observations: Vec<DocumentSymbolWriteBatchObservationInput>,
    pub route_status_completes: Vec<DocumentSymbolWriteBatchRouteStatusCompleteInput>,
    pub close_stale_nodes: Vec<DocumentSymbolWriteBatchCloseStaleRouteInput>,
    pub close_stale_edges: Vec<DocumentSymbolWriteBatchCloseStaleRouteInput>,
}
