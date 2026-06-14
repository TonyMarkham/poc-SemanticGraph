use crate::{RustAnalyzerLibError, RustAnalyzerLibResult};

use ide::{AnalysisHost, FileId, FileStructureConfig, StructureNodeKind, SymbolKind};
use ide_db::line_index::WideEncoding;
use load_cargo::{LoadCargoConfig, ProcMacroServerChoice};
use lsp_types::{DocumentSymbol, Position, Range, SymbolTag};
use paths::AbsPathBuf;
use project_model::{CargoConfig, ProjectManifest, ProjectWorkspace};
use std::path::{Path, PathBuf};
use syntax::{TextRange, TextSize};
use vfs::{FileExcluded, VfsPath};

pub fn document_symbols_for_file(
    workspace_root: impl AsRef<Path>,
    file_path: impl AsRef<Path>,
) -> RustAnalyzerLibResult<Vec<DocumentSymbol>> {
    let loaded = LoadedAnalysis::load(workspace_root.as_ref())?;
    loaded.document_symbols_for_path(file_path.as_ref())
}

pub(super) struct LoadedAnalysis {
    analysis: ide::Analysis,
    vfs: vfs::Vfs,
}

impl LoadedAnalysis {
    pub(super) fn load(workspace_root: &Path) -> RustAnalyzerLibResult<Self> {
        let workspace_root = absolute_path(workspace_root, "canonicalize workspace root")?;
        let abs_root = AbsPathBuf::assert_utf8(workspace_root);
        let manifest = ProjectManifest::discover_single(&abs_root).map_err(|source| {
            RustAnalyzerLibError::project("discover Rust project manifest for analysis", source)
        })?;
        let cargo_config = CargoConfig {
            set_test: true,
            ..CargoConfig::default()
        };
        let project_workspace =
            ProjectWorkspace::load(manifest, &cargo_config, &|_| {}).map_err(|source| {
                RustAnalyzerLibError::project(
                    "load rust-analyzer project workspace for analysis",
                    source,
                )
            })?;
        let load_config = LoadCargoConfig {
            load_out_dirs_from_check: false,
            with_proc_macro_server: ProcMacroServerChoice::None,
            prefill_caches: false,
            num_worker_threads: 1,
            proc_macro_processes: 1,
        };
        let (db, vfs, _proc_macro) =
            load_cargo::load_workspace(project_workspace, &cargo_config.extra_env, &load_config)
                .map_err(|source| {
                    RustAnalyzerLibError::project(
                        "load rust-analyzer workspace database for analysis",
                        source,
                    )
                })?;
        let host = AnalysisHost::with_database(db);

        Ok(Self {
            analysis: host.analysis(),
            vfs,
        })
    }

    pub(super) fn document_symbols_for_path(
        &self,
        file_path: &Path,
    ) -> RustAnalyzerLibResult<Vec<DocumentSymbol>> {
        let file_path = absolute_path(file_path, "canonicalize Rust source file")?;
        let file_id = self.file_id_for_path(&file_path)?;
        let line_index = self
            .analysis
            .file_line_index(file_id)
            .map_err(|source| RustAnalyzerLibError::analysis("load file line index", source))?;
        let structure_nodes = self
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

    fn file_id_for_path(&self, file_path: &Path) -> RustAnalyzerLibResult<FileId> {
        let abs_path = AbsPathBuf::assert_utf8(file_path.to_path_buf());
        let vfs_path = VfsPath::from(abs_path);
        match self.vfs.file_id(&vfs_path) {
            Some((file_id, FileExcluded::No)) => Ok(file_id),
            Some((_file_id, FileExcluded::Yes)) => Err(RustAnalyzerLibError::invalid_path(
                file_path,
                "source file is excluded from the rust-analyzer VFS",
            )),
            None => Err(RustAnalyzerLibError::invalid_path(
                file_path,
                "source file was not loaded into the rust-analyzer VFS",
            )),
        }
    }
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

fn range(line_index: &ide::LineIndex, text_range: TextRange) -> RustAnalyzerLibResult<Range> {
    Ok(Range::new(
        position(line_index, text_range.start())?,
        position(line_index, text_range.end())?,
    ))
}

fn position(line_index: &ide::LineIndex, offset: TextSize) -> RustAnalyzerLibResult<Position> {
    let line_col = line_index.line_col(offset);
    let line_col = line_index
        .to_wide(WideEncoding::Utf16, line_col)
        .ok_or_else(|| {
            RustAnalyzerLibError::analysis_message(
                "convert text offset to UTF-16",
                "offset is not on a UTF-8 character boundary",
            )
        })?;

    Ok(Position::new(line_col.line, line_col.col))
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

fn absolute_path(path: &Path, context: &'static str) -> RustAnalyzerLibResult<PathBuf> {
    path.canonicalize()
        .map_err(|source| RustAnalyzerLibError::io(context, Some(path.to_path_buf()), source))
}
