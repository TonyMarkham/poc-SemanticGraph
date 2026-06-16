use crate::model::GraphLanguage;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouteName {
    value: &'static str,
}

impl RouteName {
    pub const RUST_DOCUMENT_SYMBOLS: Self = Self {
        value: "rust.document_symbols",
    };

    pub const RUST_REFERENCES: Self = Self {
        value: "rust.references",
    };

    pub const RUST_CALLS: Self = Self {
        value: "rust.calls",
    };

    pub const CSHARP_DOCUMENT_SYMBOLS: Self = Self {
        value: "csharp.document_symbols",
    };

    pub const CSHARP_REFERENCES: Self = Self {
        value: "csharp.references",
    };

    pub const CSHARP_CALLS: Self = Self {
        value: "csharp.calls",
    };

    pub fn document_symbols_for_language(language: GraphLanguage) -> Self {
        match language {
            GraphLanguage::Rust => Self::RUST_DOCUMENT_SYMBOLS,
            GraphLanguage::CSharp => Self::CSHARP_DOCUMENT_SYMBOLS,
        }
    }

    pub fn references_for_language(language: GraphLanguage) -> Self {
        match language {
            GraphLanguage::Rust => Self::RUST_REFERENCES,
            GraphLanguage::CSharp => Self::CSHARP_REFERENCES,
        }
    }

    pub fn calls_for_language(language: GraphLanguage) -> Self {
        match language {
            GraphLanguage::Rust => Self::RUST_CALLS,
            GraphLanguage::CSharp => Self::CSHARP_CALLS,
        }
    }

    pub fn as_str(self) -> &'static str {
        self.value
    }
}
