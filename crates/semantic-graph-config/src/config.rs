use crate::{DatabaseConfig, WriterConfig};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    database: DatabaseConfig,
    writer: WriterConfig,
}

impl Config {
    pub fn new(database: DatabaseConfig, writer: WriterConfig) -> Self {
        Self { database, writer }
    }

    pub fn database(&self) -> &DatabaseConfig {
        &self.database
    }

    pub fn writer(&self) -> &WriterConfig {
        &self.writer
    }
}
