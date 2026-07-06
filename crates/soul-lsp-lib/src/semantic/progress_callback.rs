use std::sync::Arc;

pub type ProgressCallback = Arc<dyn Fn() + Send + Sync + 'static>;
