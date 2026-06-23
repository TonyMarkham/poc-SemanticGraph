use crate::{
    CSharpConfig, DatabaseConfig, ExtractorConfig, FtsConfig, QueryServiceConfig, SoulConfig,
    WriterConfig,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    database: DatabaseConfig,
    extractor: ExtractorConfig,
    writer: WriterConfig,
    query_service: QueryServiceConfig,
    csharp: CSharpConfig,
    soul: SoulConfig,
    fts: FtsConfig,
}

impl Config {
    pub fn new(
        database: DatabaseConfig,
        extractor: ExtractorConfig,
        writer: WriterConfig,
        query_service: QueryServiceConfig,
        csharp: CSharpConfig,
        soul: SoulConfig,
        fts: FtsConfig,
    ) -> Self {
        Self {
            database,
            extractor,
            writer,
            query_service,
            csharp,
            soul,
            fts,
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

    pub fn query_service(&self) -> &QueryServiceConfig {
        &self.query_service
    }

    pub fn csharp(&self) -> &CSharpConfig {
        &self.csharp
    }

    pub fn soul(&self) -> &SoulConfig {
        &self.soul
    }

    pub fn fts(&self) -> &FtsConfig {
        &self.fts
    }
}
