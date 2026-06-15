use std::{collections::BTreeMap, time::Duration};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BenchmarkSummary {
    entries: BTreeMap<String, String>,
}

impl BenchmarkSummary {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert_count(&mut self, key: &str, value: usize) {
        self.entries.insert(key.to_string(), value.to_string());
    }

    pub fn insert_duration_ms(&mut self, key: &str, value: Duration) {
        self.entries
            .insert(format!("{key}_ms"), value.as_millis().to_string());
    }

    pub fn insert_label(&mut self, key: &str, value: impl Into<String>) {
        self.entries.insert(key.to_string(), value.into());
    }

    pub fn extend_from(&mut self, source: &BenchmarkSummary) {
        for (key, value) in &source.entries {
            self.entries.insert(key.clone(), value.clone());
        }
    }

    pub fn lines(&self) -> Vec<String> {
        self.entries
            .iter()
            .map(|(key, value)| format!("bench.{key}={value}"))
            .collect()
    }
}
