mod error;
mod model;
mod row;
mod service;
mod sqlite;

// ---------------------------------------------------------------------------------------------- //

pub use error::{QueryError, QueryResult};
pub use model::{
    EdgeDetails, EdgeDetailsRequest, EdgeEndpoint, EdgeEvidence, EdgeSummary, ExtractionRunSummary,
    FileSummary, FileSummaryFile, FileSummaryRequest, FtsLineRange, FtsSearchHit, FtsSearchRequest,
    FtsSearchResults, FtsSearchSnippet, GraphPath, GraphPathStep, GraphProjection, GraphStats,
    NeighborDirection, NeighborsRequest, NodeDetails, NodeDetailsRequest, NodeNeighbor,
    NodeNeighbors, NodeOccurrence, NodeRelationSummary, NodeSearchRequest, NodeSearchResult,
    NodeSearchResults, NodeSummary, ProjectionMetadata, ProjectionRequest, RouteStatus,
    RouteStatusRequest, RouteStatusResults, ShortestPathRequest, SoulLinkedSource,
    SoulSearchRequest, SoulSearchResult, SoulSearchResults,
};
pub use service::{FtsQueryService, GraphQueryService};
