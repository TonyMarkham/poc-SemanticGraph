use std::path::PathBuf;

pub(crate) struct CliRouteBatchStorage<'a> {
    pub(crate) config: &'a Option<PathBuf>,
    pub(crate) db: Option<PathBuf>,
}

impl<'a> CliRouteBatchStorage<'a> {
    pub(crate) fn new(config: &'a Option<PathBuf>, db: Option<PathBuf>) -> Self {
        Self { config, db }
    }
}
