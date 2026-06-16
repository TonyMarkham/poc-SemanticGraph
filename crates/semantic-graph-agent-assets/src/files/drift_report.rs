use std::path::PathBuf;

#[derive(Default)]
pub(crate) struct DriftReport {
    pub(crate) missing: Vec<PathBuf>,
    pub(crate) changed: Vec<PathBuf>,
    pub(crate) stale: Vec<PathBuf>,
}

impl DriftReport {
    pub(crate) fn is_empty(&self) -> bool {
        self.missing.is_empty() && self.changed.is_empty() && self.stale.is_empty()
    }

    pub(crate) fn message(&self) -> String {
        let mut lines = Vec::new();
        lines.push("generated Codex assets drifted".to_string());
        append_paths(&mut lines, "missing", &self.missing);
        append_paths(&mut lines, "changed", &self.changed);
        append_paths(&mut lines, "stale", &self.stale);
        lines.join("\n")
    }

    pub(crate) fn sort_paths(&mut self) {
        sort_paths(&mut self.missing);
        sort_paths(&mut self.changed);
        sort_paths(&mut self.stale);
    }
}

fn append_paths(lines: &mut Vec<String>, label: &str, paths: &[PathBuf]) {
    for path in paths {
        lines.push(format!("{label}: {}", path.display()));
    }
}

fn sort_paths(paths: &mut [PathBuf]) {
    paths.sort_by_key(|path| path.to_string_lossy().to_string());
}
