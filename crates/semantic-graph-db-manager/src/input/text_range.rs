#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextRange {
    pub start_line: i64,
    pub start_col: i64,
    pub end_line: i64,
    pub end_col: i64,
}
