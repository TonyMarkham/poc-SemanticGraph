use crate::fts::FtsDiscoveredFile;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FtsDiscoveryResult {
    files: Vec<FtsDiscoveredFile>,
    scanned_files: usize,
    skipped_files: usize,
    skipped_directories: usize,
    skipped_by_config: usize,
    skipped_by_no_rust: usize,
    skipped_by_no_csharp: usize,
    skipped_by_no_submodules: usize,
}

impl FtsDiscoveryResult {
    pub fn files(&self) -> &[FtsDiscoveredFile] {
        &self.files
    }

    pub fn into_files(self) -> Vec<FtsDiscoveredFile> {
        self.files
    }

    pub fn scanned_files(&self) -> usize {
        self.scanned_files
    }

    pub fn skipped_files(&self) -> usize {
        self.skipped_files
    }

    pub fn skipped_directories(&self) -> usize {
        self.skipped_directories
    }

    pub fn skipped_by_config(&self) -> usize {
        self.skipped_by_config
    }

    pub fn skipped_by_no_rust(&self) -> usize {
        self.skipped_by_no_rust
    }

    pub fn skipped_by_no_csharp(&self) -> usize {
        self.skipped_by_no_csharp
    }

    pub fn skipped_by_no_submodules(&self) -> usize {
        self.skipped_by_no_submodules
    }

    pub(crate) fn push_file(&mut self, file: FtsDiscoveredFile) {
        self.files.push(file);
    }

    pub(crate) fn count_scanned_file(&mut self) {
        self.scanned_files += 1;
    }

    pub(crate) fn count_skipped_file(&mut self) {
        self.skipped_files += 1;
    }

    pub(crate) fn count_skipped_directory(&mut self) {
        self.skipped_directories += 1;
    }

    pub(crate) fn count_skipped_by_config(&mut self) {
        self.skipped_by_config += 1;
    }

    pub(crate) fn count_skipped_by_no_rust(&mut self) {
        self.skipped_by_no_rust += 1;
    }

    pub(crate) fn count_skipped_by_no_csharp(&mut self) {
        self.skipped_by_no_csharp += 1;
    }

    pub(crate) fn count_skipped_by_no_submodules(&mut self) {
        self.skipped_by_no_submodules += 1;
    }
}
