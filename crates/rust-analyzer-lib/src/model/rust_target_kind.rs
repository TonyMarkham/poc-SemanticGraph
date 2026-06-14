#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RustTargetKind {
    Lib,
    Bin,
    Example,
    Test,
    Bench,
    BuildScript,
    Other,
}
