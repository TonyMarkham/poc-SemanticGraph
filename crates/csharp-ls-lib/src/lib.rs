mod error;
mod lsp;
mod model;
mod project;
mod semantic;
#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------------------------- //

pub use error::{CSharpLsLibError, CSharpLsLibResult};
pub use model::{
    CSharpProjectMatch, CSharpProjectModel, CSharpSolutionModel, FileSemanticResult,
    FileSemanticWork, ResolvedCallTarget, ResolvedIncomingCall, ResolvedIncomingCallSet,
    ResolvedReferenceLocation, ResolvedReferenceSet, ResolvedReferenceTarget,
};
pub use project::{load_solution, project_for_file, project_source_files, solution_source_files};
pub use semantic::{CSharpLsWorker, CSharpLsWorkerPool, provider_version};
