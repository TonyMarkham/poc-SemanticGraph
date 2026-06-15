use lsp_types::DocumentSymbol;
use std::path::PathBuf;

pub type DocumentSymbolItems = Vec<(PathBuf, Vec<DocumentSymbol>)>;
