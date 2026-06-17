#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FtsSkipReason {
    Config,
    NoRust,
    NoCSharp,
    NoSubmodules,
    BinaryOrUnreadable,
}
