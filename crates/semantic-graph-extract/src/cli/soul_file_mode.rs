#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoulFileMode {
    Full,
    Symbols,
    References,
}

impl SoulFileMode {
    pub fn includes_symbols(self) -> bool {
        matches!(self, Self::Full | Self::Symbols)
    }

    pub fn includes_references(self) -> bool {
        matches!(self, Self::Full | Self::References)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Symbols => "symbols",
            Self::References => "references",
        }
    }
}
