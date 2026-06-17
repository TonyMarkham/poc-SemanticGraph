use crate::{
    RouteWriteBatchEdgeEvidenceInput, RouteWriteBatchEdgeInput, RouteWriteBatchObservationInput,
    RouteWriteBatchOccurrenceInput,
};

#[derive(Debug, Clone, Default)]
pub struct RouteWriteBatchInput {
    pub edges: Vec<RouteWriteBatchEdgeInput>,
    pub occurrences: Vec<RouteWriteBatchOccurrenceInput>,
    pub edge_evidence: Vec<RouteWriteBatchEdgeEvidenceInput>,
    pub route_observations: Vec<RouteWriteBatchObservationInput>,
}
