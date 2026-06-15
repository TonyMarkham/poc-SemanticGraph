use crate::{Config, DbManagerError, DbManagerResult, WriteHandle, write_worker::WriteWorker};

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use std::{fs, path::Path, time::Duration};
use tokio::sync::{broadcast, mpsc};

pub struct WriteManager;

impl WriteManager {
    pub async fn start(path: impl AsRef<Path>) -> DbManagerResult<WriteHandle> {
        Self::start_with_config(path, Config::default()).await
    }

    pub async fn start_with_config(
        path: impl AsRef<Path>,
        config: Config,
    ) -> DbManagerResult<WriteHandle> {
        let path = path.as_ref();
        create_database_parent(path)?;

        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .foreign_keys(true)
            .busy_timeout(Duration::from_millis(config.busy_timeout_ms()))
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .map_err(DbManagerError::database)?;

        let (sender, receiver) = mpsc::channel(config.queue_capacity());
        let (progress, _unused_receiver) = broadcast::channel(config.queue_capacity().max(1));
        let worker = WriteWorker::new(pool, receiver, progress.clone());
        let worker_task = tokio::spawn(async move {
            worker.run().await;
        });

        Ok(WriteHandle::new(sender, progress, worker_task))
    }
}

fn create_database_parent(path: &Path) -> DbManagerResult<()> {
    let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    else {
        return Ok(());
    };

    fs::create_dir_all(parent).map_err(|source| {
        DbManagerError::io(
            "create database parent directory",
            Some(parent.to_path_buf()),
            source,
        )
    })
}
