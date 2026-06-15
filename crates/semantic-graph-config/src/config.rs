use crate::DatabaseConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    database: DatabaseConfig,
}

impl Config {
    pub fn new(database: DatabaseConfig) -> Self {
        Self { database }
    }

    pub fn database(&self) -> &DatabaseConfig {
        &self.database
    }
}
