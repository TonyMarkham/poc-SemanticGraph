use crate::{
    RustAnalyzerLibError, RustAnalyzerLibResult,
    model::{ResolvedCallTarget, ResolvedOutgoingCall, ResolvedOutgoingCallSet},
    semantic::{loaded_analysis::LoadedAnalysis, lsp_range::range},
};

use ide::{CallHierarchyConfig, FilePosition, RaFixtureConfig};
use ide_db::line_index::{WideEncoding, WideLineCol};
use lsp_types::Position;
use std::path::Path;
use syntax::TextSize;

pub fn outgoing_calls_for_symbols(
    workspace_root: impl AsRef<Path>,
    callers: &[ResolvedCallTarget],
) -> RustAnalyzerLibResult<Vec<ResolvedOutgoingCallSet>> {
    let loaded = LoadedAnalysis::load(workspace_root.as_ref())?;
    let mut call_sets = Vec::with_capacity(callers.len());

    for caller in callers {
        call_sets.push(outgoing_calls_for_target(&loaded, caller)?);
    }

    Ok(call_sets)
}

fn outgoing_calls_for_target(
    loaded: &LoadedAnalysis,
    caller: &ResolvedCallTarget,
) -> RustAnalyzerLibResult<ResolvedOutgoingCallSet> {
    let caller_file_id = loaded.file_id_for_path(&caller.file_path)?;
    let caller_line_index = loaded
        .analysis
        .file_line_index(caller_file_id)
        .map_err(|source| RustAnalyzerLibError::analysis("load caller file line index", source))?;
    let seed_position = FilePosition {
        file_id: caller_file_id,
        offset: text_size_for_position(&caller_line_index, caller.selection_range.start)?,
    };
    let config = CallHierarchyConfig {
        exclude_tests: false,
        ra_fixture: RaFixtureConfig::default(),
    };
    let prepared_items = loaded
        .analysis
        .call_hierarchy(seed_position, &config)
        .map_err(|source| RustAnalyzerLibError::analysis("prepare call hierarchy", source))?
        .map(|range_info| range_info.info)
        .unwrap_or_default();
    let mut skipped_non_callable_prepare_items = 0;
    let mut outgoing_calls = Vec::new();

    for item in prepared_items {
        if !matches!(
            item.kind,
            Some(ide_db::SymbolKind::Function | ide_db::SymbolKind::Method)
        ) {
            skipped_non_callable_prepare_items += 1;
            continue;
        }

        let item_position = FilePosition {
            file_id: item.file_id,
            offset: item.focus_or_full_range().start(),
        };
        let Some(call_items) = loaded
            .analysis
            .outgoing_calls(&config, item_position)
            .map_err(|source| RustAnalyzerLibError::analysis("extract outgoing calls", source))?
        else {
            continue;
        };

        for call_item in call_items {
            let Some(target_file_path) = loaded.file_path_for_id(call_item.target.file_id) else {
                continue;
            };
            let target_line_index = loaded
                .analysis
                .file_line_index(call_item.target.file_id)
                .map_err(|source| {
                    RustAnalyzerLibError::analysis("load call target file line index", source)
                })?;
            let target_range = range(&target_line_index, call_item.target.full_range)?;
            let target_selection_range =
                range(&target_line_index, call_item.target.focus_or_full_range())?;
            let target_kind = call_item
                .target
                .kind
                .map(symbol_kind_name)
                .unwrap_or("function")
                .to_string();
            let mut callsite_ranges = Vec::new();

            for callsite_range in call_item.ranges {
                if callsite_range.file_id != caller_file_id {
                    continue;
                }
                callsite_ranges.push(range(&caller_line_index, callsite_range.range)?);
            }

            if callsite_ranges.is_empty() {
                continue;
            }

            outgoing_calls.push(ResolvedOutgoingCall {
                target_file_path,
                target_range,
                target_selection_range,
                target_name: call_item.target.name.to_string(),
                target_kind,
                callsite_ranges,
            });
        }
    }

    outgoing_calls.sort_by(|left, right| {
        left.target_file_path
            .cmp(&right.target_file_path)
            .then(
                left.target_selection_range
                    .start
                    .line
                    .cmp(&right.target_selection_range.start.line),
            )
            .then(
                left.target_selection_range
                    .start
                    .character
                    .cmp(&right.target_selection_range.start.character),
            )
            .then(left.target_name.cmp(&right.target_name))
    });

    Ok(ResolvedOutgoingCallSet {
        caller_file_path: caller.file_path.clone(),
        caller_selection_range: caller.selection_range,
        caller_name: caller.name.clone(),
        outgoing_calls,
        skipped_non_callable_prepare_items,
    })
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

fn symbol_kind_name(kind: ide_db::SymbolKind) -> &'static str {
    match kind {
        ide_db::SymbolKind::Attribute
        | ide_db::SymbolKind::BuiltinAttr
        | ide_db::SymbolKind::Derive
        | ide_db::SymbolKind::DeriveHelper
        | ide_db::SymbolKind::Macro
        | ide_db::SymbolKind::ProcMacro => "function",
        ide_db::SymbolKind::Const | ide_db::SymbolKind::ConstParam | ide_db::SymbolKind::Static => {
            "constant"
        }
        ide_db::SymbolKind::CrateRoot => "package",
        ide_db::SymbolKind::Enum => "enum",
        ide_db::SymbolKind::Field => "field",
        ide_db::SymbolKind::Function => "function",
        ide_db::SymbolKind::Method => "method",
        ide_db::SymbolKind::Impl => "object",
        ide_db::SymbolKind::InlineAsmRegOrRegClass
        | ide_db::SymbolKind::Label
        | ide_db::SymbolKind::LifetimeParam
        | ide_db::SymbolKind::Local
        | ide_db::SymbolKind::SelfParam
        | ide_db::SymbolKind::ValueParam => "variable",
        ide_db::SymbolKind::Module | ide_db::SymbolKind::ToolModule => "module",
        ide_db::SymbolKind::SelfType
        | ide_db::SymbolKind::TypeAlias
        | ide_db::SymbolKind::TypeParam => "type_parameter",
        ide_db::SymbolKind::Struct | ide_db::SymbolKind::Union => "struct",
        ide_db::SymbolKind::Trait => "interface",
        ide_db::SymbolKind::Variant => "enum_member",
    }
}
