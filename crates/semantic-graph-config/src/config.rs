use crate::{CSharpConfig, DatabaseConfig, ExtractorConfig, WriterConfig};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    database: DatabaseConfig,
    extractor: ExtractorConfig,
    writer: WriterConfig,
    csharp: CSharpConfig,
}

impl Config {
    pub fn new(
        database: DatabaseConfig,
        extractor: ExtractorConfig,
        writer: WriterConfig,
        csharp: CSharpConfig,
    ) -> Self {
        Self {
            database,
            extractor,
            writer,
            csharp,
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

    pub fn csharp(&self) -> &CSharpConfig {
        &self.csharp
    }
}
