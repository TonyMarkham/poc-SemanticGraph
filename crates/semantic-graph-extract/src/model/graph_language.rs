#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphLanguage {
    Rust,
    CSharp,
    Soul,
}

impl GraphLanguage {
    pub fn as_store_str(self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::CSharp => "csharp",
            Self::Soul => "soul",
        }
    }

    pub fn workspace_kind(self) -> &'static str {
        self.as_store_str()
    }
}
