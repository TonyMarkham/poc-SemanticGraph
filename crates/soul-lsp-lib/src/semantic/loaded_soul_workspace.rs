use crate::{
    SoulLspConfig, SoulLspLibError, SoulLspLibResult,
    model::{ResolvedReferenceLocation, ResolvedReferenceSet, ResolvedReferenceTarget},
    semantic::{DocumentSymbolItems, FileSemanticResult, FileSemanticWork, ProgressCallback},
};

use indexer::{
    CodeAnnotation, Document, PluginRegistry, SemanticGraph,
    config::{PluginEntry, ScanConfig, SoulConfig, load_config},
    markdown::wikilink_at_position,
    scan_repository,
};
use lsp_types::{DocumentSymbol, Position, Range, SymbolKind};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

pub struct LoadedSoulWorkspace {
    workspace_root: PathBuf,
    graph: SemanticGraph,
}

impl LoadedSoulWorkspace {
    pub fn load(workspace_root: impl AsRef<Path>) -> SoulLspLibResult<Self> {
        let workspace_root = canonical_workspace_root(workspace_root.as_ref())?;
        let config = load_config(&workspace_root)
            .map_err(|source| SoulLspLibError::project("load Soul config", source))?;
        Self::load_scanned_workspace(workspace_root, config)
    }

    pub fn load_with_config(
        workspace_root: impl AsRef<Path>,
        config: SoulLspConfig,
    ) -> SoulLspLibResult<Self> {
        let workspace_root = canonical_workspace_root(workspace_root.as_ref())?;
        Self::load_scanned_workspace(workspace_root, indexer_config_from(config))
    }

    fn load_scanned_workspace(
        workspace_root: PathBuf,
        config: SoulConfig,
    ) -> SoulLspLibResult<Self> {
        let registry = PluginRegistry::load(&config.plugins, &workspace_root)
            .map_err(|source| SoulLspLibError::project("load Soul annotation plugins", source))?;
        let graph = scan_repository(&workspace_root, &config, &registry)
            .map_err(|source| SoulLspLibError::project("scan Soul workspace", source))?;

        Ok(Self {
            workspace_root,
            graph,
        })
    }

    pub fn document_symbols_for_files(
        &self,
        file_paths: Vec<PathBuf>,
    ) -> SoulLspLibResult<DocumentSymbolItems> {
        self.document_symbols_for_files_internal(file_paths, None)
    }

    pub fn document_symbols_for_files_with_progress(
        &self,
        file_paths: Vec<PathBuf>,
        progress: ProgressCallback,
    ) -> SoulLspLibResult<DocumentSymbolItems> {
        self.document_symbols_for_files_internal(file_paths, Some(progress))
    }

    fn document_symbols_for_files_internal(
        &self,
        file_paths: Vec<PathBuf>,
        progress: Option<ProgressCallback>,
    ) -> SoulLspLibResult<DocumentSymbolItems> {
        file_paths
            .into_iter()
            .map(|file_path| {
                self.validate_file_path(&file_path)?;
                self.document_symbols_for_file(&file_path).map(|symbols| {
                    if let Some(progress) = &progress {
                        progress();
                    }
                    (file_path, symbols)
                })
            })
            .collect()
    }

    pub fn source_files(&self) -> Vec<PathBuf> {
        let mut files = BTreeSet::new();
        files.extend(
            self.graph
                .documents
                .iter()
                .map(|document| self.absolute_graph_path(&document.path)),
        );
        files.extend(
            self.graph
                .annotations
                .iter()
                .map(|annotation| self.absolute_graph_path(&annotation.path)),
        );
        files.extend(
            self.graph
                .references
                .iter()
                .map(|reference| self.absolute_graph_path(&reference.source_path)),
        );

        files.into_iter().collect()
    }

    pub fn document_symbols_for_file(
        &self,
        file_path: impl AsRef<Path>,
    ) -> SoulLspLibResult<Vec<DocumentSymbol>> {
        let file_path = file_path.as_ref();
        self.validate_file_path(file_path)?;

        let annotations = self
            .graph
            .annotations
            .iter()
            .filter(|annotation| self.path_matches(&annotation.path, file_path))
            .map(document_symbol_for_annotation)
            .collect::<Vec<_>>();

        let references = self
            .graph
            .references
            .iter()
            .filter(|reference| self.path_matches(&reference.source_path, file_path))
            .filter_map(document_symbol_for_reference)
            .collect::<Vec<_>>();

        if let Some(document) = self.document_at(file_path) {
            let mut children = annotations;
            children.extend(references);
            Ok(vec![document_symbol_for_document(document, children)])
        } else {
            Ok(annotations.into_iter().chain(references).collect())
        }
    }

    pub fn references_for_symbol(
        &self,
        target: ResolvedReferenceTarget,
    ) -> SoulLspLibResult<ResolvedReferenceSet> {
        self.validate_file_path(&target.file_path)?;
        let target_id = self
            .resolved_id_at_position(&target.file_path, target.selection_range.start)?
            .ok_or_else(|| {
                SoulLspLibError::analysis_message(
                    "resolve Soul reference target",
                    format!(
                        "no Soul document, annotation, or wikilink resolved at {:?}:{:?}",
                        target.file_path, target.selection_range.start
                    ),
                )
            })?;

        let mut references = self
            .graph
            .annotations
            .iter()
            .filter(|annotation| annotation.id == target_id)
            .filter_map(|annotation| self.annotation_location(annotation))
            .collect::<Vec<_>>();

        references.extend(
            self.graph
                .references
                .iter()
                .filter(|reference| reference.target_id == target_id)
                .filter_map(|reference| self.reference_location(reference)),
        );

        Ok(ResolvedReferenceSet {
            target_file_path: target.file_path,
            target_selection_range: target.selection_range,
            target_name: target.name,
            references,
        })
    }

    pub fn file_semantic_work(
        &self,
        work: FileSemanticWork,
    ) -> SoulLspLibResult<FileSemanticResult> {
        let reference_sets = work
            .reference_targets
            .into_iter()
            .map(|target| self.references_for_symbol(target))
            .collect::<SoulLspLibResult<Vec<_>>>()?;

        Ok(FileSemanticResult {
            file_path: work.file_path,
            reference_sets,
        })
    }

    fn validate_file_path(&self, file_path: &Path) -> SoulLspLibResult<()> {
        if !file_path.is_file() {
            return Err(SoulLspLibError::invalid_path(
                file_path,
                "Soul analysis requires an existing file path",
            ));
        }

        let canonical = file_path.canonicalize().map_err(|source| {
            SoulLspLibError::io(
                "canonicalize Soul file path",
                Some(file_path.to_path_buf()),
                source,
            )
        })?;
        if !canonical.starts_with(&self.workspace_root) {
            return Err(SoulLspLibError::invalid_path(
                canonical,
                "Soul file path is outside the workspace root",
            ));
        }

        Ok(())
    }

    fn resolved_id_at_position(
        &self,
        file_path: &Path,
        position: Position,
    ) -> SoulLspLibResult<Option<String>> {
        let text = fs::read_to_string(file_path).map_err(|source| {
            SoulLspLibError::io(
                "read Soul source file for position lookup",
                Some(file_path.to_path_buf()),
                source,
            )
        })?;
        if let Some(token) = wikilink_at_position(
            &text,
            (position.line as usize) + 1,
            position.character as usize,
        ) {
            return Ok(Some(token.target_id));
        }

        if let Some(annotation) = self.annotation_at(file_path, position.line) {
            return Ok(Some(annotation.id.clone()));
        }

        Ok(self
            .document_at(file_path)
            .map(|document| document.id.clone()))
    }

    fn annotation_at(&self, file_path: &Path, line: u32) -> Option<&CodeAnnotation> {
        let target = (line as usize) + 1;
        self.graph.annotations.iter().find(|annotation| {
            annotation.line == target && self.path_matches(&annotation.path, file_path)
        })
    }

    fn document_at(&self, file_path: &Path) -> Option<&Document> {
        self.graph
            .documents
            .iter()
            .find(|document| self.path_matches(&document.path, file_path))
    }

    fn annotation_location(
        &self,
        annotation: &CodeAnnotation,
    ) -> Option<ResolvedReferenceLocation> {
        Some(ResolvedReferenceLocation {
            file_path: self.absolute_graph_path(&annotation.path),
            range: indexed_line_range(annotation.line),
        })
    }

    fn reference_location(
        &self,
        reference: &indexer::Reference,
    ) -> Option<ResolvedReferenceLocation> {
        Some(ResolvedReferenceLocation {
            file_path: self.absolute_graph_path(&reference.source_path),
            range: range_for_reference(reference)?,
        })
    }

    fn path_matches(&self, graph_path: &Path, file_path: &Path) -> bool {
        let left = self.absolute_graph_path(graph_path);
        let left = left.canonicalize().unwrap_or(left);
        let right = file_path
            .canonicalize()
            .unwrap_or_else(|_| file_path.to_path_buf());

        left == right
    }

    fn absolute_graph_path(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.workspace_root.join(path)
        }
    }
}

fn indexer_config_from(config: SoulLspConfig) -> SoulConfig {
    SoulConfig {
        scan: ScanConfig {
            excluded_dirs: config.scan().excluded_dirs().to_vec(),
            excluded_dir_suffixes: config.scan().excluded_dir_suffixes().to_vec(),
            excluded_bin_except_under: config.scan().excluded_bin_except_under().to_vec(),
        },
        plugins: config
            .plugins()
            .iter()
            .map(|plugin| PluginEntry {
                language: plugin.language().to_string(),
                path: plugin.path().clone(),
            })
            .collect(),
    }
}

fn canonical_workspace_root(workspace_root: &Path) -> SoulLspLibResult<PathBuf> {
    let canonical = workspace_root.canonicalize().map_err(|source| {
        SoulLspLibError::io(
            "canonicalize Soul workspace root",
            Some(workspace_root.to_path_buf()),
            source,
        )
    })?;
    if !canonical.is_dir() {
        return Err(SoulLspLibError::invalid_path(
            canonical,
            "Soul workspace root must be a directory",
        ));
    }

    Ok(canonical)
}

#[allow(deprecated)]
fn document_symbol_for_document(
    document: &Document,
    children: Vec<DocumentSymbol>,
) -> DocumentSymbol {
    let range = point_range(0);
    DocumentSymbol {
        name: document.id.clone(),
        detail: document
            .title
            .clone()
            .or_else(|| Some(document.kind.clone())),
        kind: SymbolKind::FILE,
        tags: None,
        deprecated: None,
        range,
        selection_range: range,
        children: if children.is_empty() {
            None
        } else {
            Some(children)
        },
    }
}

#[allow(deprecated)]
fn document_symbol_for_annotation(annotation: &CodeAnnotation) -> DocumentSymbol {
    let range = indexed_line_range(annotation.line);
    DocumentSymbol {
        name: annotation.id.clone(),
        detail: Some(annotation.syntax.to_string()),
        kind: SymbolKind::OBJECT,
        tags: None,
        deprecated: None,
        range,
        selection_range: range,
        children: None,
    }
}

fn document_symbol_for_reference(reference: &indexer::Reference) -> Option<DocumentSymbol> {
    let range = range_for_reference(reference)?;
    let has_display_text = reference.display_text.is_some();
    #[allow(deprecated)]
    Some(DocumentSymbol {
        name: reference
            .display_text
            .clone()
            .unwrap_or_else(|| reference.target_id.clone()),
        detail: has_display_text.then(|| reference.target_id.clone()),
        kind: SymbolKind::STRING,
        tags: None,
        deprecated: None,
        range,
        selection_range: range,
        children: None,
    })
}

fn range_for_reference(reference: &indexer::Reference) -> Option<Range> {
    if reference.source_end_line < reference.source_start_line {
        return None;
    }
    if reference.source_end_line == reference.source_start_line
        && reference.source_end_col < reference.source_start_col
    {
        return None;
    }

    Some(Range {
        start: Position {
            line: indexed_line(reference.source_start_line),
            character: to_u32_saturating(reference.source_start_col),
        },
        end: Position {
            line: indexed_line(reference.source_end_line),
            character: to_u32_saturating(reference.source_end_col),
        },
    })
}

fn indexed_line_range(line: usize) -> Range {
    point_range(indexed_line(line))
}

fn point_range(line: u32) -> Range {
    Range {
        start: Position { line, character: 0 },
        end: Position { line, character: 0 },
    }
}

fn indexed_line(line: usize) -> u32 {
    to_u32_saturating(line.saturating_sub(1))
}

fn to_u32_saturating(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}
