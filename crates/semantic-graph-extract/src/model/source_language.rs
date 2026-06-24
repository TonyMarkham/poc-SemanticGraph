use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceLanguage {
    Rust,
    CSharp,
    Markdown,
    Other,
}

impl SourceLanguage {
    pub fn as_store_str(self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::CSharp => "csharp",
            Self::Markdown => "markdown",
            Self::Other => "other",
        }
    }

    pub fn from_path(path: &Path) -> Self {
        match path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("rs") => Self::Rust,
            Some("cs" | "csx") => Self::CSharp,
            Some("md" | "markdown" | "mdx") => Self::Markdown,
            _ => Self::Other,
        }
    }
}
