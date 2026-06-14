use crate::projection::GraphProjectionService;

use std::{path::PathBuf, sync::Arc};

#[derive(Clone)]
pub struct AppState {
    projection_service: Arc<GraphProjectionService>,
}

impl AppState {
    pub fn new(database_path: PathBuf) -> Self {
        Self {
            projection_service: Arc::new(GraphProjectionService::new(database_path)),
        }
    }

    pub fn projection_service(&self) -> &GraphProjectionService {
        &self.projection_service
    }
}
