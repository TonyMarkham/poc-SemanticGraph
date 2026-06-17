use crate::{
    DbManagerResult, DemoSeedSummary, DocumentSymbolWriteBatchInput,
    DocumentSymbolWriteBatchSummary, RouteWriteBatchInput, StaleFileSummary, WriteSummary,
    models::{
        OwnedCloseStaleFileInput, OwnedCloseStaleFtsDocumentsInput, OwnedCloseStaleRouteInput,
        OwnedEdgeEvidenceInput, OwnedEdgeInput, OwnedFileInput, OwnedFtsDocumentInput,
        OwnedNodeInput, OwnedOccurrenceInput, OwnedRouteObservationInput,
        OwnedRouteStatusCompleteInput, OwnedRouteStatusFailInput, OwnedRouteStatusStartInput,
    },
};

use tokio::sync::oneshot;

pub(crate) enum Commands {
    Migrate {
        response: oneshot::Sender<DbManagerResult<()>>,
    },
    CreateWorkspace {
        root_uri: String,
        kind: String,
        response: oneshot::Sender<DbManagerResult<i64>>,
    },
    WorkspaceId {
        root_uri: String,
        response: oneshot::Sender<DbManagerResult<Option<i64>>>,
    },
    FileId {
        workspace_id: i64,
        uri: String,
        response: oneshot::Sender<DbManagerResult<Option<i64>>>,
    },
    NodeExists {
        node_id: String,
        response: oneshot::Sender<DbManagerResult<bool>>,
    },
    StartRun {
        workspace_id: i64,
        provider: String,
        provider_version: Option<String>,
        git_commit: Option<String>,
        response: oneshot::Sender<DbManagerResult<i64>>,
    },
    FinishRun {
        run_id: i64,
        status: String,
        response: oneshot::Sender<DbManagerResult<()>>,
    },
    UpsertFile {
        input: OwnedFileInput,
        response: oneshot::Sender<DbManagerResult<i64>>,
    },
    UpsertFtsDocument {
        input: OwnedFtsDocumentInput,
        response: oneshot::Sender<DbManagerResult<i64>>,
    },
    UpsertNode {
        input: OwnedNodeInput,
        response: oneshot::Sender<DbManagerResult<String>>,
    },
    UpsertEdge {
        input: OwnedEdgeInput,
        response: oneshot::Sender<DbManagerResult<String>>,
    },
    InsertOccurrence {
        input: OwnedOccurrenceInput,
        response: oneshot::Sender<DbManagerResult<i64>>,
    },
    InsertEdgeEvidence {
        input: OwnedEdgeEvidenceInput,
        response: oneshot::Sender<DbManagerResult<i64>>,
    },
    StartRouteStatus {
        input: OwnedRouteStatusStartInput,
        response: oneshot::Sender<DbManagerResult<i64>>,
    },
    CompleteRouteStatus {
        input: OwnedRouteStatusCompleteInput,
        response: oneshot::Sender<DbManagerResult<()>>,
    },
    FailRouteStatus {
        input: OwnedRouteStatusFailInput,
        response: oneshot::Sender<DbManagerResult<()>>,
    },
    RecordRouteObservation {
        input: OwnedRouteObservationInput,
        response: oneshot::Sender<DbManagerResult<()>>,
    },
    WriteRouteBatch {
        input: RouteWriteBatchInput,
        response: oneshot::Sender<DbManagerResult<()>>,
    },
    WriteDocumentSymbolBatch {
        input: DocumentSymbolWriteBatchInput,
        response: oneshot::Sender<DbManagerResult<DocumentSymbolWriteBatchSummary>>,
    },
    CloseStaleNodesForRoute {
        input: OwnedCloseStaleRouteInput,
        response: oneshot::Sender<DbManagerResult<u64>>,
    },
    CloseStaleFile {
        input: OwnedCloseStaleFileInput,
        response: oneshot::Sender<DbManagerResult<StaleFileSummary>>,
    },
    CloseStaleFtsDocumentsForWorkspace {
        input: OwnedCloseStaleFtsDocumentsInput,
        response: oneshot::Sender<DbManagerResult<u64>>,
    },
    CloseStaleEdgesForRoute {
        input: OwnedCloseStaleRouteInput,
        response: oneshot::Sender<DbManagerResult<u64>>,
    },
    DemoSeed {
        root_uri: String,
        response: oneshot::Sender<DbManagerResult<DemoSeedSummary>>,
    },
    Shutdown {
        response: oneshot::Sender<DbManagerResult<WriteSummary>>,
    },
}

impl Commands {
    pub(crate) fn name(&self) -> &'static str {
        match self {
            Self::Migrate { .. } => "migrate",
            Self::CreateWorkspace { .. } => "create_workspace",
            Self::WorkspaceId { .. } => "workspace_id",
            Self::FileId { .. } => "file_id",
            Self::NodeExists { .. } => "node_exists",
            Self::StartRun { .. } => "start_run",
            Self::FinishRun { .. } => "finish_run",
            Self::UpsertFile { .. } => "upsert_file",
            Self::UpsertFtsDocument { .. } => "upsert_fts_document",
            Self::UpsertNode { .. } => "upsert_node",
            Self::UpsertEdge { .. } => "upsert_edge",
            Self::InsertOccurrence { .. } => "insert_occurrence",
            Self::InsertEdgeEvidence { .. } => "insert_edge_evidence",
            Self::StartRouteStatus { .. } => "start_route_status",
            Self::CompleteRouteStatus { .. } => "complete_route_status",
            Self::FailRouteStatus { .. } => "fail_route_status",
            Self::RecordRouteObservation { .. } => "record_route_observation",
            Self::WriteRouteBatch { .. } => "write_route_batch",
            Self::WriteDocumentSymbolBatch { .. } => "write_document_symbol_batch",
            Self::CloseStaleNodesForRoute { .. } => "close_stale_nodes_for_route",
            Self::CloseStaleFile { .. } => "close_stale_file",
            Self::CloseStaleFtsDocumentsForWorkspace { .. } => {
                "close_stale_fts_documents_for_workspace"
            }
            Self::CloseStaleEdgesForRoute { .. } => "close_stale_edges_for_route",
            Self::DemoSeed { .. } => "demo_seed",
            Self::Shutdown { .. } => "shutdown",
        }
    }

    pub(crate) fn is_write(&self) -> bool {
        !matches!(
            self,
            Self::WorkspaceId { .. }
                | Self::FileId { .. }
                | Self::NodeExists { .. }
                | Self::Shutdown { .. }
        )
    }
}
