mod error;
mod model;
mod project;
mod semantic;
#[cfg(test)]
mod tests;
mod version;

// ---------------------------------------------------------------------------------------------- //

pub use error::{RustAnalyzerLibError, RustAnalyzerLibResult};
pub use model::{
    ResolvedCallTarget, ResolvedOutgoingCall, ResolvedOutgoingCallSet, ResolvedReferenceLocation,
    ResolvedReferenceSet, ResolvedReferenceTarget, RustPackage, RustSourceFile, RustTarget,
    RustTargetKind, RustWorkspaceModel,
};
pub use project::{load_package, load_workspace, package_source_files, workspace_source_files};
pub use semantic::{
    AnalysisWorker, AnalysisWorkerHandle, AnalysisWorkerPool, DocumentSymbolItems,
    FileSemanticResult, FileSemanticWork, LoadedAnalysis, SharedAnalysisHost,
    SharedAnalysisSnapshot, SharedAnalysisWorker, SharedAnalysisWorkerHandle,
    SharedAnalysisWorkerPool, document_symbols_for_file, document_symbols_for_files,
    outgoing_calls_for_symbols, references_for_symbols,
};
pub use version::provider_version;
