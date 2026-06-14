mod error;
mod model;
mod project;
mod semantic;
#[cfg(test)]
mod tests;
mod version;

// ---------------------------------------------------------------------------------------------- //

pub use error::{RustAnalyzerLibError, RustAnalyzerLibResult};
pub use project::{load_package, load_workspace, package_source_files, workspace_source_files};
pub use semantic::{document_symbols_for_file, document_symbols_for_files};
pub use version::provider_version;
