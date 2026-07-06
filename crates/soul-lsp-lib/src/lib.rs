mod config;
mod error;
mod model;
mod semantic;
#[cfg(test)]
mod tests;
mod version;

// ---------------------------------------------------------------------------------------------- //

pub use config::{SoulLspConfig, SoulLspPluginConfig, SoulLspScanConfig};
pub use error::{SoulLspLibError, SoulLspLibResult};
pub use model::{ResolvedReferenceLocation, ResolvedReferenceSet, ResolvedReferenceTarget};
pub use semantic::{
    AnalysisWorker, AnalysisWorkerHandle, DocumentSymbolItems, FileSemanticResult,
    FileSemanticWork, LoadedSoulWorkspace, ProgressCallback, document_symbols_for_file,
    document_symbols_for_files, document_symbols_for_files_with_progress, references_for_symbols,
};
pub use version::provider_version;
