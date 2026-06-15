use crate::{
    RustAnalyzerLibResult,
    model::{
        ResolvedCallTarget, ResolvedOutgoingCallSet, ResolvedReferenceSet, ResolvedReferenceTarget,
    },
    semantic::{DocumentSymbolItems, FileSemanticResult, FileSemanticWork},
};

use std::path::PathBuf;
use tokio::sync::oneshot;

pub(crate) enum AnalysisWorkerCommand {
    DocumentSymbols {
        file_paths: Vec<PathBuf>,
        response: oneshot::Sender<RustAnalyzerLibResult<DocumentSymbolItems>>,
    },
    References {
        target: ResolvedReferenceTarget,
        response: oneshot::Sender<RustAnalyzerLibResult<ResolvedReferenceSet>>,
    },
    OutgoingCalls {
        target: ResolvedCallTarget,
        response: oneshot::Sender<RustAnalyzerLibResult<ResolvedOutgoingCallSet>>,
    },
    FileSemantic {
        work: FileSemanticWork,
        response: oneshot::Sender<RustAnalyzerLibResult<FileSemanticResult>>,
    },
    Shutdown {
        response: oneshot::Sender<RustAnalyzerLibResult<()>>,
    },
}
