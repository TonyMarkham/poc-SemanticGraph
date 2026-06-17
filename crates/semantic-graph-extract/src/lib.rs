pub mod benchmark;
pub mod cli;
pub mod document_symbols;
pub mod error;
pub mod fts;
pub mod lsp_stdio;
pub mod model;
pub mod persist;
pub mod providers;
#[cfg(test)]
mod tests;
pub mod workspace_extraction;

// ---------------------------------------------------------------------------------------------- //

pub use error::{ExtractError, ExtractResult};
