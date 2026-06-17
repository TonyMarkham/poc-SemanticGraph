mod fts_discovered_file;
mod fts_discovery_result;
mod fts_exclusion_set;
mod fts_extraction_options;
mod fts_extraction_runner;
mod fts_extraction_summary;
mod fts_file_discovery;
mod fts_file_language;
mod fts_skip_reason;

pub use fts_discovered_file::FtsDiscoveredFile;
pub use fts_discovery_result::FtsDiscoveryResult;
pub use fts_exclusion_set::FtsExclusionSet;
pub use fts_extraction_options::FtsExtractionOptions;
pub use fts_extraction_runner::FtsExtractionRunner;
pub use fts_extraction_summary::FtsExtractionSummary;
pub use fts_file_discovery::FtsFileDiscovery;
pub use fts_file_language::FtsFileLanguage;
pub use fts_skip_reason::FtsSkipReason;

use std::path::Path;

pub const FTS_PROVIDER: &str = "semantic-graph-extract";
pub const FTS_ROUTE: &str = "fts.full_text";
pub const FTS_SCOPE: &str = "workspace";

pub fn normalize_relative_path(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            std::path::Component::Normal(value) => Some(value.to_string_lossy().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}
