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

    pub fn as_str(self) -> &'static str {
        self.value
    }
}
