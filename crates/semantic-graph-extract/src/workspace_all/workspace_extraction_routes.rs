#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkspaceExtractionRoutes {
    symbols: bool,
    references: bool,
    calls: bool,
}

impl WorkspaceExtractionRoutes {
    pub fn all() -> Self {
        Self {
            symbols: true,
            references: true,
            calls: true,
        }
    }

    pub fn from_selectors(symbols: bool, references: bool, calls: bool) -> Self {
        if symbols || references || calls {
            Self {
                symbols,
                references,
                calls,
            }
        } else {
            Self::all()
        }
    }

    pub fn includes_symbols(self) -> bool {
        self.symbols
    }

    pub fn includes_references(self) -> bool {
        self.references
    }

    pub fn includes_calls(self) -> bool {
        self.calls
    }

    pub fn includes_relations(self) -> bool {
        self.references || self.calls
    }

    pub fn label(self) -> &'static str {
        match (self.symbols, self.references, self.calls) {
            (true, true, true) => "all",
            (true, false, false) => "symbols",
            (false, true, false) => "references",
            (false, false, true) => "calls",
            (true, true, false) => "symbols+references",
            (true, false, true) => "symbols+calls",
            (false, true, true) => "references+calls",
            (false, false, false) => "none",
        }
    }
}
