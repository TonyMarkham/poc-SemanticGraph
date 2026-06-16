use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderedAsset {
    output_path: PathBuf,
    content: String,
}

impl RenderedAsset {
    pub fn new(output_path: PathBuf, content: String) -> Self {
        Self {
            output_path,
            content: ensure_trailing_newline(content),
        }
    }

    pub fn output_path(&self) -> &PathBuf {
        &self.output_path
    }

    pub fn content(&self) -> &str {
        &self.content
    }
}

fn ensure_trailing_newline(mut content: String) -> String {
    while content.ends_with("\n\n") {
        content.pop();
    }
    if !content.ends_with('\n') {
        content.push('\n');
    }
    content
}
