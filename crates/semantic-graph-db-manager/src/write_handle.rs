use crate::{
    ActiveFileSymbols, CloseStaleFileInput, CloseStaleFtsDocumentsInput, CloseStaleRouteInput,
    DbManagerError, DbManagerResult, DemoSeedSummary, DocumentSymbolWriteBatchInput,
    DocumentSymbolWriteBatchSummary, EdgeEvidenceInput, EdgeInput, FileInput, FtsDocumentInput,
    FtsWriteBatchInput, NodeInput, OccurrenceInput, RouteObservationInput,
    RouteStatusCompleteInput, RouteStatusFailInput, RouteStatusStartInput, RouteWriteBatchInput,
    StaleFileSummary, WriteProgress, WriteSummary, commands::Commands,
};

use std::collections::HashMap;
use std::sync::Arc;
use tokio::{
    sync::{Mutex, broadcast, mpsc, oneshot},
    task::JoinHandle,
};

#[derive(Debug, Clone)]
pub struct WriteHandle {
    sender: mpsc::Sender<Commands>,
    progress: broadcast::Sender<WriteProgress>,
    worker_task: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl WriteHandle {
    pub(crate) fn new(
        sender: mpsc::Sender<Commands>,
        progress: broadcast::Sender<WriteProgress>,
        worker_task: JoinHandle<()>,
    ) -> Self {
        Self {
            sender,
            progress,
            worker_task: Arc::new(Mutex::new(Some(worker_task))),
        }
    }

    pub fn subscribe_progress(&self) -> broadcast::Receiver<WriteProgress> {
        self.progress.subscribe()
    }

    pub async fn migrate(&self) -> DbManagerResult<()> {
        let (response, receiver) = oneshot::channel();
        self.sender.send(Commands::Migrate { response }).await?;
        receiver.await?
    }

    pub async fn create_workspace(&self, root_uri: &str, kind: &str) -> DbManagerResult<i64> {
        let (response, receiver) = oneshot::channel();
        self.sender
            .send(Commands::CreateWorkspace {
                root_uri: root_uri.to_string(),
                kind: kind.to_string(),
                response,
            })
            .await?;
        receiver.await?
    }

    pub async fn workspace_id(&self, root_uri: &str) -> DbManagerResult<Option<i64>> {
        let (response, receiver) = oneshot::channel();
        self.sender
            .send(Commands::WorkspaceId {
                root_uri: root_uri.to_string(),
                response,
            })
            .await?;
        receiver.await?
    }

    pub async fn file_id(&self, workspace_id: i64, uri: &str) -> DbManagerResult<Option<i64>> {
        let (response, receiver) = oneshot::channel();
        self.sender
            .send(Commands::FileId {
                workspace_id,
                uri: uri.to_string(),
                response,
            })
            .await?;
        receiver.await?
    }

    pub async fn file_route_content_hashes(
        &self,
        workspace_id: i64,
        route: &str,
        provider: &str,
    ) -> DbManagerResult<HashMap<String, Option<String>>> {
        let (response, receiver) = oneshot::channel();
        self.sender
            .send(Commands::FileRouteContentHashes {
                workspace_id,
                route: route.to_string(),
                provider: provider.to_string(),
                response,
            })
            .await?;
        receiver.await?
    }

    pub async fn active_fts_document_hashes(
        &self,
        workspace_id: i64,
    ) -> DbManagerResult<HashMap<String, String>> {
        let (response, receiver) = oneshot::channel();
        self.sender
            .send(Commands::ActiveFtsDocumentHashes {
                workspace_id,
                response,
            })
            .await?;
        receiver.await?
    }

    pub async fn active_file_symbols(
        &self,
        workspace_id: i64,
        file_uris: &[String],
    ) -> DbManagerResult<Vec<ActiveFileSymbols>> {
        let (response, receiver) = oneshot::channel();
        self.sender
            .send(Commands::ActiveFileSymbols {
                workspace_id,
                file_uris: file_uris.to_vec(),
                response,
            })
            .await?;
        receiver.await?
    }

    pub async fn node_exists(&self, node_id: &str) -> DbManagerResult<bool> {
        let (response, receiver) = oneshot::channel();
        self.sender
            .send(Commands::NodeExists {
                node_id: node_id.to_string(),
                response,
            })
            .await?;
        receiver.await?
    }

    pub async fn start_run(
        &self,
        workspace_id: i64,
        provider: &str,
        provider_version: Option<&str>,
        git_commit: Option<&str>,
    ) -> DbManagerResult<i64> {
        let (response, receiver) = oneshot::channel();
        self.sender
            .send(Commands::StartRun {
                workspace_id,
                provider: provider.to_string(),
                provider_version: provider_version.map(str::to_string),
                git_commit: git_commit.map(str::to_string),
                response,
            })
            .await?;
        receiver.await?
    }

    pub async fn finish_run(&self, run_id: i64, status: &str) -> DbManagerResult<()> {
        let (response, receiver) = oneshot::channel();
        self.sender
            .send(Commands::FinishRun {
                run_id,
                status: status.to_string(),
                response,
            })
            .await?;
        receiver.await?
    }

    pub async fn upsert_file(&self, input: FileInput<'_>) -> DbManagerResult<i64> {
        let (response, receiver) = oneshot::channel();
        self.sender
            .send(Commands::UpsertFile {
                input: input.into(),
                response,
            })
            .await?;
        receiver.await?
    }

    pub async fn upsert_fts_document(&self, input: FtsDocumentInput<'_>) -> DbManagerResult<i64> {
        let (response, receiver) = oneshot::channel();
        self.sender
            .send(Commands::UpsertFtsDocument {
                input: input.into(),
                response,
            })
            .await?;
        receiver.await?
    }

    pub async fn upsert_node(&self, input: NodeInput<'_>) -> DbManagerResult<String> {
        let (response, receiver) = oneshot::channel();
        self.sender
            .send(Commands::UpsertNode {
                input: input.into(),
                response,
            })
            .await?;
        receiver.await?
    }

    pub async fn upsert_edge(&self, input: EdgeInput<'_>) -> DbManagerResult<String> {
        let (response, receiver) = oneshot::channel();
        self.sender
            .send(Commands::UpsertEdge {
                input: input.into(),
                response,
            })
            .await?;
        receiver.await?
    }

    pub async fn insert_occurrence(&self, input: OccurrenceInput<'_>) -> DbManagerResult<i64> {
        let (response, receiver) = oneshot::channel();
        self.sender
            .send(Commands::InsertOccurrence {
                input: input.into(),
                response,
            })
            .await?;
        receiver.await?
    }

    pub async fn insert_edge_evidence(&self, input: EdgeEvidenceInput<'_>) -> DbManagerResult<i64> {
        let (response, receiver) = oneshot::channel();
        self.sender
            .send(Commands::InsertEdgeEvidence {
                input: input.into(),
                response,
            })
            .await?;
        receiver.await?
    }

    pub async fn start_route_status(
        &self,
        input: RouteStatusStartInput<'_>,
    ) -> DbManagerResult<i64> {
        let (response, receiver) = oneshot::channel();
        self.sender
            .send(Commands::StartRouteStatus {
                input: input.into(),
                response,
            })
            .await?;
        receiver.await?
    }

    pub async fn complete_route_status(
        &self,
        input: RouteStatusCompleteInput<'_>,
    ) -> DbManagerResult<()> {
        let (response, receiver) = oneshot::channel();
        self.sender
            .send(Commands::CompleteRouteStatus {
                input: input.into(),
                response,
            })
            .await?;
        receiver.await?
    }

    pub async fn fail_route_status(&self, input: RouteStatusFailInput<'_>) -> DbManagerResult<()> {
        let (response, receiver) = oneshot::channel();
        self.sender
            .send(Commands::FailRouteStatus {
                input: input.into(),
                response,
            })
            .await?;
        receiver.await?
    }

    pub async fn record_route_observation(
        &self,
        input: RouteObservationInput<'_>,
    ) -> DbManagerResult<()> {
        let (response, receiver) = oneshot::channel();
        self.sender
            .send(Commands::RecordRouteObservation {
                input: input.into(),
                response,
            })
            .await?;
        receiver.await?
    }

    pub async fn write_route_batch(&self, input: RouteWriteBatchInput) -> DbManagerResult<()> {
        let (response, receiver) = oneshot::channel();
        self.sender
            .send(Commands::WriteRouteBatch { input, response })
            .await?;
        receiver.await?
    }

    pub async fn write_document_symbol_batch(
        &self,
        input: DocumentSymbolWriteBatchInput,
    ) -> DbManagerResult<DocumentSymbolWriteBatchSummary> {
        let (response, receiver) = oneshot::channel();
        self.sender
            .send(Commands::WriteDocumentSymbolBatch { input, response })
            .await?;
        receiver.await?
    }

    pub async fn write_fts_batch(&self, input: FtsWriteBatchInput) -> DbManagerResult<()> {
        let (response, receiver) = oneshot::channel();
        self.sender
            .send(Commands::WriteFtsBatch { input, response })
            .await?;
        receiver.await?
    }

    pub async fn write_fts_content_batch(&self, input: FtsWriteBatchInput) -> DbManagerResult<()> {
        let (response, receiver) = oneshot::channel();
        self.sender
            .send(Commands::WriteFtsContentBatch { input, response })
            .await?;
        receiver.await?
    }

    pub async fn close_stale_nodes_for_route(
        &self,
        input: CloseStaleRouteInput<'_>,
    ) -> DbManagerResult<u64> {
        let (response, receiver) = oneshot::channel();
        self.sender
            .send(Commands::CloseStaleNodesForRoute {
                input: input.into(),
                response,
            })
            .await?;
        receiver.await?
    }

    pub async fn close_stale_edges_for_route(
        &self,
        input: CloseStaleRouteInput<'_>,
    ) -> DbManagerResult<u64> {
        let (response, receiver) = oneshot::channel();
        self.sender
            .send(Commands::CloseStaleEdgesForRoute {
                input: input.into(),
                response,
            })
            .await?;
        receiver.await?
    }

    pub async fn close_stale_edges_for_route_source_file(
        &self,
        input: CloseStaleRouteInput<'_>,
    ) -> DbManagerResult<u64> {
        let (response, receiver) = oneshot::channel();
        self.sender
            .send(Commands::CloseStaleEdgesForRouteSourceFile {
                input: input.into(),
                response,
            })
            .await?;
        receiver.await?
    }

    pub async fn close_stale_file(
        &self,
        input: CloseStaleFileInput<'_>,
    ) -> DbManagerResult<StaleFileSummary> {
        let (response, receiver) = oneshot::channel();
        self.sender
            .send(Commands::CloseStaleFile {
                input: input.into(),
                response,
            })
            .await?;
        receiver.await?
    }

    pub async fn close_stale_fts_documents_for_workspace(
        &self,
        input: CloseStaleFtsDocumentsInput<'_>,
    ) -> DbManagerResult<u64> {
        let (response, receiver) = oneshot::channel();
        self.sender
            .send(Commands::CloseStaleFtsDocumentsForWorkspace {
                input: input.into(),
                response,
            })
            .await?;
        receiver.await?
    }

    pub async fn demo_seed(&self, root_uri: &str) -> DbManagerResult<DemoSeedSummary> {
        let (response, receiver) = oneshot::channel();
        self.sender
            .send(Commands::DemoSeed {
                root_uri: root_uri.to_string(),
                response,
            })
            .await?;
        receiver.await?
    }

    pub async fn shutdown(&self) -> DbManagerResult<WriteSummary> {
        let (response, receiver) = oneshot::channel();
        if let Err(error) = self.sender.send(Commands::Shutdown { response }).await {
            self.await_worker_task().await?;
            return Err(error.into());
        }

        let response_result = receiver.await;
        self.await_worker_task().await?;
        response_result?
    }

    async fn await_worker_task(&self) -> DbManagerResult<()> {
        let mut worker_task = self.worker_task.lock().await;
        let Some(worker_task) = worker_task.take() else {
            return Ok(());
        };

        worker_task.await.map_err(DbManagerError::worker_task)
    }
}
