use crate::{RustAnalyzerLibError, RustAnalyzerLibResult};

use ide_db::line_index::WideEncoding;
use lsp_types::{Position, Range};
use syntax::{TextRange, TextSize};

pub(super) fn range(
    line_index: &ide::LineIndex,
    text_range: TextRange,
) -> RustAnalyzerLibResult<Range> {
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
