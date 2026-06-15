use crate::{DatabaseConfig, ExtractorConfig, WriterConfig};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    database: DatabaseConfig,
    extractor: ExtractorConfig,
    writer: WriterConfig,
}

impl Config {
    pub fn new(database: DatabaseConfig, extractor: ExtractorConfig, writer: WriterConfig) -> Self {
        Self {
            database,
            extractor,
            writer,
        }
    }

    pub fn database(&self) -> &DatabaseConfig {
        &self.database
    }

    pub fn extractor(&self) -> &ExtractorConfig {
        &self.extractor
    }

    pub fn writer(&self) -> &WriterConfig {
        &self.writer
    }
}
