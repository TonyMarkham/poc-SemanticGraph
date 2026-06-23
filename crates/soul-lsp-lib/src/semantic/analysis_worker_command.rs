use crate::{
    SoulLspLibResult,
    model::{ResolvedReferenceSet, ResolvedReferenceTarget},
    semantic::{DocumentSymbolItems, FileSemanticResult, FileSemanticWork},
};

use std::path::PathBuf;
use tokio::sync::oneshot;

pub(crate) enum AnalysisWorkerCommand {
    DocumentSymbols {
        file_paths: Vec<PathBuf>,
        response: oneshot::Sender<SoulLspLibResult<DocumentSymbolItems>>,
    },
    References {
        target: ResolvedReferenceTarget,
        response: oneshot::Sender<SoulLspLibResult<ResolvedReferenceSet>>,
    },
    FileSemantic {
        work: FileSemanticWork,
        response: oneshot::Sender<SoulLspLibResult<FileSemanticResult>>,
    },
    Shutdown {
        response: oneshot::Sender<SoulLspLibResult<()>>,
    },
}
