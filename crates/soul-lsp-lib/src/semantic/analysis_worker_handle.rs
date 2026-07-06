use crate::{
    SoulLspLibError, SoulLspLibResult,
    model::{ResolvedReferenceSet, ResolvedReferenceTarget},
    semantic::analysis_worker_command::AnalysisWorkerCommand,
    semantic::{DocumentSymbolItems, FileSemanticResult, FileSemanticWork, ProgressCallback},
};

use std::{path::PathBuf, sync::mpsc};
use tokio::sync::oneshot;

#[derive(Clone)]
pub struct AnalysisWorkerHandle {
    sender: mpsc::Sender<AnalysisWorkerCommand>,
}

impl AnalysisWorkerHandle {
    pub(crate) fn new(sender: mpsc::Sender<AnalysisWorkerCommand>) -> Self {
        Self { sender }
    }

    pub async fn document_symbols_for_files(
        &self,
        file_paths: Vec<PathBuf>,
    ) -> SoulLspLibResult<DocumentSymbolItems> {
        self.document_symbols_for_files_internal(file_paths, None)
            .await
    }

    pub async fn document_symbols_for_files_with_progress(
        &self,
        file_paths: Vec<PathBuf>,
        progress: ProgressCallback,
    ) -> SoulLspLibResult<DocumentSymbolItems> {
        self.document_symbols_for_files_internal(file_paths, Some(progress))
            .await
    }

    async fn document_symbols_for_files_internal(
        &self,
        file_paths: Vec<PathBuf>,
        progress: Option<ProgressCallback>,
    ) -> SoulLspLibResult<DocumentSymbolItems> {
        let (response, receiver) = oneshot::channel();
        self.sender
            .send(AnalysisWorkerCommand::DocumentSymbols {
                file_paths,
                progress,
                response,
            })
            .map_err(|_| {
                SoulLspLibError::analysis_message(
                    "send analysis worker command",
                    "analysis worker closed",
                )
            })?;
        receiver.await.map_err(|_| {
            SoulLspLibError::analysis_message(
                "receive analysis worker response",
                "analysis worker closed before responding",
            )
        })?
    }

    pub async fn references_for_symbol(
        &self,
        target: ResolvedReferenceTarget,
    ) -> SoulLspLibResult<ResolvedReferenceSet> {
        let (response, receiver) = oneshot::channel();
        self.sender
            .send(AnalysisWorkerCommand::References { target, response })
            .map_err(|_| {
                SoulLspLibError::analysis_message(
                    "send analysis worker command",
                    "analysis worker closed",
                )
            })?;
        receiver.await.map_err(|_| {
            SoulLspLibError::analysis_message(
                "receive analysis worker response",
                "analysis worker closed before responding",
            )
        })?
    }

    pub async fn file_semantic_work(
        &self,
        work: FileSemanticWork,
    ) -> SoulLspLibResult<FileSemanticResult> {
        let (response, receiver) = oneshot::channel();
        self.sender
            .send(AnalysisWorkerCommand::FileSemantic { work, response })
            .map_err(|_| {
                SoulLspLibError::analysis_message(
                    "send analysis worker command",
                    "analysis worker closed",
                )
            })?;
        receiver.await.map_err(|_| {
            SoulLspLibError::analysis_message(
                "receive analysis worker response",
                "analysis worker closed before responding",
            )
        })?
    }

    pub async fn shutdown(&self) -> SoulLspLibResult<()> {
        let (response, receiver) = oneshot::channel();
        self.sender
            .send(AnalysisWorkerCommand::Shutdown { response })
            .map_err(|_| {
                SoulLspLibError::analysis_message(
                    "send analysis worker command",
                    "analysis worker closed",
                )
            })?;
        receiver.await.map_err(|_| {
            SoulLspLibError::analysis_message(
                "receive analysis worker shutdown response",
                "analysis worker closed before responding",
            )
        })?
    }
}
