#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RustFileMode {
    Full,
    Symbols,
    References,
    Calls,
}

impl RustFileMode {
    pub fn includes_symbols(self) -> bool {
        matches!(self, Self::Full | Self::Symbols)
    }

    pub fn includes_references(self) -> bool {
        matches!(self, Self::Full | Self::References)
    }

    pub fn includes_calls(self) -> bool {
        matches!(self, Self::Full | Self::Calls)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Symbols => "symbols",
            Self::References => "references",
            Self::Calls => "calls",
        }
    }
}
