use crate::{
    RustAnalyzerLibError, RustAnalyzerLibResult,
    semantic::{loaded_analysis::LoadedAnalysis, loaded_analysis::absolute_path, lsp_range::range},
};

use ide::{FileStructureConfig, StructureNodeKind, SymbolKind};
use lsp_types::{DocumentSymbol, SymbolTag};
use std::path::Path;

pub fn document_symbols_for_file(
    workspace_root: impl AsRef<Path>,
    file_path: impl AsRef<Path>,
) -> RustAnalyzerLibResult<Vec<DocumentSymbol>> {
    let loaded = LoadedAnalysis::load(workspace_root.as_ref())?;
    document_symbols_for_path(&loaded, file_path.as_ref())
}

pub(super) fn document_symbols_for_path(
    loaded: &LoadedAnalysis,
    file_path: &Path,
) -> RustAnalyzerLibResult<Vec<DocumentSymbol>> {
    let file_path = absolute_path(file_path, "canonicalize Rust source file")?;
    let file_id = loaded.file_id_for_path(&file_path)?;
    let line_index = loaded
        .analysis
        .file_line_index(file_id)
        .map_err(|source| RustAnalyzerLibError::analysis("load file line index", source))?;
    let structure_nodes = loaded
        .analysis
        .file_structure(
            &FileStructureConfig {
                exclude_locals: true,
            },
            file_id,
        )
        .map_err(|source| RustAnalyzerLibError::analysis("extract file structure", source))?;

    let mut symbols = Vec::with_capacity(structure_nodes.len());
    for node in structure_nodes {
        let tags = if node.deprecated {
            vec![SymbolTag::DEPRECATED]
        } else {
            Vec::new()
        };

        #[allow(deprecated)]
        let symbol = DocumentSymbol {
            name: node.label,
            detail: node.detail,
            kind: structure_node_kind(node.kind),
            tags: Some(tags),
            deprecated: Some(node.deprecated),
            range: range(&line_index, node.node_range)?,
            selection_range: range(&line_index, node.navigation_range)?,
            children: None,
        };
        symbols.push((symbol, node.parent));
    }

    Ok(build_hierarchy(symbols))
}

fn build_hierarchy(mut symbols: Vec<(DocumentSymbol, Option<usize>)>) -> Vec<DocumentSymbol> {
    let mut roots = Vec::new();
    while let Some((mut symbol, parent_index)) = symbols.pop() {
        if let Some(children) = &mut symbol.children {
            children.reverse();
        }
        let parent = match parent_index {
            None => &mut roots,
            Some(index) => symbols[index].0.children.get_or_insert_with(Vec::new),
        };
        parent.push(symbol);
    }
    roots.reverse();
    roots
}

fn structure_node_kind(kind: StructureNodeKind) -> lsp_types::SymbolKind {
    match kind {
        StructureNodeKind::SymbolKind(symbol) => symbol_kind(symbol),
        StructureNodeKind::Region | StructureNodeKind::ExternBlock => {
            lsp_types::SymbolKind::NAMESPACE
        }
    }
}

fn symbol_kind(symbol_kind: SymbolKind) -> lsp_types::SymbolKind {
    match symbol_kind {
        SymbolKind::Function => lsp_types::SymbolKind::FUNCTION,
        SymbolKind::Method => lsp_types::SymbolKind::METHOD,
        SymbolKind::Struct => lsp_types::SymbolKind::STRUCT,
        SymbolKind::Enum => lsp_types::SymbolKind::ENUM,
        SymbolKind::Variant => lsp_types::SymbolKind::ENUM_MEMBER,
        SymbolKind::Trait => lsp_types::SymbolKind::INTERFACE,
        SymbolKind::Macro
        | SymbolKind::ProcMacro
        | SymbolKind::BuiltinAttr
        | SymbolKind::Attribute
        | SymbolKind::Derive
        | SymbolKind::DeriveHelper => lsp_types::SymbolKind::FUNCTION,
        SymbolKind::CrateRoot => lsp_types::SymbolKind::PACKAGE,
        SymbolKind::Module | SymbolKind::ToolModule => lsp_types::SymbolKind::MODULE,
        SymbolKind::TypeAlias | SymbolKind::TypeParam | SymbolKind::SelfType => {
            lsp_types::SymbolKind::TYPE_PARAMETER
        }
        SymbolKind::Field => lsp_types::SymbolKind::FIELD,
        SymbolKind::Static | SymbolKind::Const | SymbolKind::ConstParam => {
            lsp_types::SymbolKind::CONSTANT
        }
        SymbolKind::Impl => lsp_types::SymbolKind::OBJECT,
        SymbolKind::Local
        | SymbolKind::SelfParam
        | SymbolKind::LifetimeParam
        | SymbolKind::ValueParam
        | SymbolKind::Label
        | SymbolKind::InlineAsmRegOrRegClass => lsp_types::SymbolKind::VARIABLE,
        SymbolKind::Union => lsp_types::SymbolKind::STRUCT,
    }
}
