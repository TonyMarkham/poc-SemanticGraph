use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FtsFileLanguage {
    Rust,
    CSharp,
    Markdown,
    Other,
}

impl FtsFileLanguage {
    pub fn from_path(path: &Path) -> Self {
        let extension = path
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();

        match extension.as_str() {
            "rs" => Self::Rust,
            "cs" => Self::CSharp,
            "md" | "markdown" | "mdx" => Self::Markdown,
            _ => Self::Other,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::CSharp => "csharp",
            Self::Markdown => "markdown",
            Self::Other => "other",
        }
    }
}
