use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProviderId(&'static str);

impl ProviderId {
    pub const fn new(value: &'static str) -> Self {
        Self(value)
    }

    pub const fn rust_analyzer() -> Self {
        Self("rust-analyzer")
    }

    pub const fn csharp_language_server() -> Self {
        Self("csharp-language-server")
    }

    pub const fn soul_lsp() -> Self {
        Self("soul-lsp")
    }

    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

impl fmt::Display for ProviderId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}
