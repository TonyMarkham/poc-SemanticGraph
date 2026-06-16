use std::path::PathBuf;

#[derive(Clone, Debug, Default)]
pub struct AssetWriteReport {
    pub created: Vec<PathBuf>,
    pub updated: Vec<PathBuf>,
    pub unchanged: Vec<PathBuf>,
    pub removed: Vec<PathBuf>,
}

impl AssetWriteReport {
    pub fn lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        lines.push(format!(
            "generated Codex assets: created={} updated={} unchanged={} removed={}",
            self.created.len(),
            self.updated.len(),
            self.unchanged.len(),
            self.removed.len()
        ));
        append_paths(&mut lines, "created", &self.created);
        append_paths(&mut lines, "updated", &self.updated);
        append_paths(&mut lines, "unchanged", &self.unchanged);
        append_paths(&mut lines, "removed", &self.removed);
        lines
    }

    pub(crate) fn sort_paths(&mut self) {
        self.created
            .sort_by_key(|path| path.to_string_lossy().to_string());
        self.updated
            .sort_by_key(|path| path.to_string_lossy().to_string());
        self.unchanged
            .sort_by_key(|path| path.to_string_lossy().to_string());
        self.removed
            .sort_by_key(|path| path.to_string_lossy().to_string());
    }
}

fn append_paths(lines: &mut Vec<String>, label: &str, paths: &[PathBuf]) {
    for path in paths {
        lines.push(format!("{label}: {}", path.display()));
    }
}
