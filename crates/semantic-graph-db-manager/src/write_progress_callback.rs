use std::sync::Arc;

pub type DbWriteProgressCallback = Arc<dyn Fn() + Send + Sync + 'static>;
