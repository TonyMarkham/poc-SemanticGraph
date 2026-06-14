use crate::{
    RustAnalyzerLibError, RustAnalyzerLibResult,
    model::{ResolvedReferenceLocation, ResolvedReferenceSet, ResolvedReferenceTarget},
    semantic::document_symbols_for_file::{LoadedAnalysis, range},
};

use ide::{FilePosition, FindAllRefsConfig, RaFixtureConfig};
use ide_db::line_index::{WideEncoding, WideLineCol};
use lsp_types::{Position, Range};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use syntax::{TextRange, TextSize};

pub fn references_for_symbols(
    workspace_root: impl AsRef<Path>,
    targets: &[ResolvedReferenceTarget],
) -> RustAnalyzerLibResult<Vec<ResolvedReferenceSet>> {
    let loaded = LoadedAnalysis::load(workspace_root.as_ref())?;
    let mut reference_sets = Vec::with_capacity(targets.len());

    for target in targets {
        reference_sets.push(loaded.references_for_target(target)?);
    }

    Ok(reference_sets)
}

impl LoadedAnalysis {
    fn references_for_target(
        &self,
        target: &ResolvedReferenceTarget,
    ) -> RustAnalyzerLibResult<ResolvedReferenceSet> {
        let file_id = self.file_id_for_path(&target.file_path)?;
        let line_index = self
            .analysis
            .file_line_index(file_id)
            .map_err(|source| RustAnalyzerLibError::analysis("load file line index", source))?;
        let position = FilePosition {
            file_id,
            offset: text_size_for_position(&line_index, target.selection_range.start)?,
        };
        let Some(search_results) = self
            .analysis
            .find_all_refs(
                position,
                &FindAllRefsConfig {
                    search_scope: None,
                    ra_fixture: RaFixtureConfig::default(),
                    exclude_imports: false,
                    exclude_tests: false,
                },
            )
            .map_err(|source| RustAnalyzerLibError::analysis("find all references", source))?
        else {
            return Ok(ResolvedReferenceSet {
                target_file_path: target.file_path.clone(),
                target_selection_range: target.selection_range,
                target_name: target.name.clone(),
                references: Vec::new(),
            });
        };

        let target_text_range = text_range_for_lsp_range(&line_index, target.selection_range)?;
        let mut seen = HashSet::new();
        let mut references = Vec::new();

        for search_result in search_results {
            for (reference_file_id, reference_ranges) in search_result.references {
                let reference_file_path = self.file_path_for_id(reference_file_id)?;
                let reference_line_index = self
                    .analysis
                    .file_line_index(reference_file_id)
                    .map_err(|source| {
                        RustAnalyzerLibError::analysis("load reference file line index", source)
                    })?;

                for (reference_range, _category) in reference_ranges {
                    if reference_file_id == file_id && reference_range == target_text_range {
                        continue;
                    }

                    let lsp_range = range(&reference_line_index, reference_range)?;
                    let dedupe_key = (
                        reference_file_path.clone(),
                        lsp_range.start.line,
                        lsp_range.start.character,
                        lsp_range.end.line,
                        lsp_range.end.character,
                    );
                    if seen.insert(dedupe_key) {
                        references.push(ResolvedReferenceLocation {
                            file_path: reference_file_path.clone(),
                            range: lsp_range,
                        });
                    }
                }
            }
        }

        references.sort_by(|left, right| {
            left.file_path
                .cmp(&right.file_path)
                .then(left.range.start.line.cmp(&right.range.start.line))
                .then(left.range.start.character.cmp(&right.range.start.character))
                .then(left.range.end.line.cmp(&right.range.end.line))
                .then(left.range.end.character.cmp(&right.range.end.character))
        });

        Ok(ResolvedReferenceSet {
            target_file_path: target.file_path.clone(),
            target_selection_range: target.selection_range,
            target_name: target.name.clone(),
            references,
        })
    }

    fn file_path_for_id(&self, file_id: ide::FileId) -> RustAnalyzerLibResult<PathBuf> {
        let vfs_path = self.vfs.file_path(file_id);
        let Some(abs_path) = vfs_path.as_path() else {
            return Err(RustAnalyzerLibError::analysis_message(
                "resolve reference file path",
                "reference file is virtual and has no filesystem path",
            ));
        };
        Ok(PathBuf::from(abs_path.to_path_buf()))
    }
}

fn text_range_for_lsp_range(
    line_index: &ide::LineIndex,
    lsp_range: Range,
) -> RustAnalyzerLibResult<TextRange> {
    let start = text_size_for_position(line_index, lsp_range.start)?;
    let end = text_size_for_position(line_index, lsp_range.end)?;
    if end < start {
        return Err(RustAnalyzerLibError::analysis_message(
            "convert UTF-16 range to text range",
            "range end is before start",
        ));
    }

    Ok(TextRange::new(start, end))
}

fn text_size_for_position(
    line_index: &ide::LineIndex,
    position: Position,
) -> RustAnalyzerLibResult<TextSize> {
    let wide_line_col = WideLineCol {
        line: position.line,
        col: position.character,
    };
    let line_col = line_index
        .to_utf8(WideEncoding::Utf16, wide_line_col)
        .ok_or_else(|| {
            RustAnalyzerLibError::analysis_message(
                "convert UTF-16 position to text offset",
                "position is not on a UTF-16 character boundary",
            )
        })?;
    let line_range = line_index.line(line_col.line).ok_or_else(|| {
        RustAnalyzerLibError::analysis_message(
            "convert UTF-16 position to text offset",
            "position line is outside the file",
        )
    })?;
    let column = TextSize::from(line_col.col);

    Ok(line_range.start() + column.min(line_range.len()))
}
