use crate::{
    RustAnalyzerLibError, RustAnalyzerLibResult,
    model::{
        ResolvedCallTarget, ResolvedOutgoingCallSet, ResolvedReferenceSet, ResolvedReferenceTarget,
    },
    semantic::analysis_worker_command::AnalysisWorkerCommand,
    semantic::{DocumentSymbolItems, FileSemanticResult, FileSemanticWork},
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
    ) -> RustAnalyzerLibResult<DocumentSymbolItems> {
        let (response, receiver) = oneshot::channel();
        self.sender
            .send(AnalysisWorkerCommand::DocumentSymbols {
                file_paths,
                response,
            })
            .map_err(|_| {
                RustAnalyzerLibError::analysis_message(
                    "send analysis worker command",
                    "analysis worker closed",
                )
            })?;
        receiver.await.map_err(|_| {
            RustAnalyzerLibError::analysis_message(
                "receive analysis worker response",
                "analysis worker closed before responding",
            )
        })?
    }

    pub async fn references_for_symbol(
        &self,
        target: ResolvedReferenceTarget,
    ) -> RustAnalyzerLibResult<ResolvedReferenceSet> {
        let (response, receiver) = oneshot::channel();
        self.sender
            .send(AnalysisWorkerCommand::References { target, response })
            .map_err(|_| {
                RustAnalyzerLibError::analysis_message(
                    "send analysis worker command",
                    "analysis worker closed",
                )
            })?;
        receiver.await.map_err(|_| {
            RustAnalyzerLibError::analysis_message(
                "receive analysis worker response",
                "analysis worker closed before responding",
            )
        })?
    }

    pub async fn outgoing_calls_for_symbol(
        &self,
        target: ResolvedCallTarget,
    ) -> RustAnalyzerLibResult<ResolvedOutgoingCallSet> {
        let (response, receiver) = oneshot::channel();
        self.sender
            .send(AnalysisWorkerCommand::OutgoingCalls { target, response })
            .map_err(|_| {
                RustAnalyzerLibError::analysis_message(
                    "send analysis worker command",
                    "analysis worker closed",
                )
            })?;
        receiver.await.map_err(|_| {
            RustAnalyzerLibError::analysis_message(
                "receive analysis worker response",
                "analysis worker closed before responding",
            )
        })?
    }

    pub async fn file_semantic_work(
        &self,
        work: FileSemanticWork,
    ) -> RustAnalyzerLibResult<FileSemanticResult> {
        let (response, receiver) = oneshot::channel();
        self.sender
            .send(AnalysisWorkerCommand::FileSemantic { work, response })
            .map_err(|_| {
                RustAnalyzerLibError::analysis_message(
                    "send analysis worker command",
                    "analysis worker closed",
                )
            })?;
        receiver.await.map_err(|_| {
            RustAnalyzerLibError::analysis_message(
                "receive analysis worker response",
                "analysis worker closed before responding",
            )
        })?
    }

    pub async fn shutdown(&self) -> RustAnalyzerLibResult<()> {
        let (response, receiver) = oneshot::channel();
        self.sender
            .send(AnalysisWorkerCommand::Shutdown { response })
            .map_err(|_| {
                RustAnalyzerLibError::analysis_message(
                    "send analysis worker command",
                    "analysis worker closed",
                )
            })?;
        receiver.await.map_err(|_| {
            RustAnalyzerLibError::analysis_message(
                "receive analysis worker shutdown response",
                "analysis worker closed before responding",
            )
        })?
    }
}
