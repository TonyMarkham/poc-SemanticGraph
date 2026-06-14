use crate::query::GraphQueryService;

use std::{path::PathBuf, sync::Arc};

#[derive(Clone)]
pub struct AppState {
    query_service: Arc<GraphQueryService>,
}

impl AppState {
    pub fn new(database_path: PathBuf) -> Self {
        Self {
            query_service: Arc::new(GraphQueryService::new(database_path)),
        }
    }

    pub fn query_service(&self) -> &GraphQueryService {
        &self.query_service
    }
}
