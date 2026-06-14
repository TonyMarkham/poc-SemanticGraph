use crate::{VisualizerServerError, VisualizerServerResult};

use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use std::path::Path;

pub(crate) async fn open_read_only_pool(path: &Path) -> VisualizerServerResult<SqlitePool> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .read_only(true)
        .foreign_keys(true);

    SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .map_err(VisualizerServerError::database)
}
