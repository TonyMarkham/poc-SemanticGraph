use thiserror::Error;

pub type Result<T> = std::result::Result<T, GraphStoreError>;

#[derive(Debug, Error)]
pub enum GraphStoreError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
}
