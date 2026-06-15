use crate::{
    CloseStaleRouteInput, DbManagerResult, DemoSeedSummary, EdgeEvidenceInput, EdgeInput,
    FileInput, NodeInput, OccurrenceInput, RouteObservationInput, RouteStatusCompleteInput,
    RouteStatusFailInput, RouteStatusStartInput, WriteProgress, WriteSummary, commands::Commands,
};

use tokio::sync::{broadcast, mpsc, oneshot};

#[derive(Debug, Clone)]
pub struct WriteHandle {
    sender: mpsc::Sender<Commands>,
    progress: broadcast::Sender<WriteProgress>,
}

impl WriteHandle {
    pub(crate) fn new(
        sender: mpsc::Sender<Commands>,
        progress: broadcast::Sender<WriteProgress>,
    ) -> Self {
        Self { sender, progress }
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
        self.sender.send(Commands::Shutdown { response }).await?;
        receiver.await?
    }
}
