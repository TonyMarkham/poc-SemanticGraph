use error_location::ErrorLocation;
use std::{io, panic::Location, path::PathBuf};
use tantivy::query::QueryParserError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TantivySearchError {
    #[error("io error during {context} path={path:?} at {location}")]
    Io {
        context: String,
        path: Option<PathBuf>,
        #[source]
        source: io::Error,
        location: ErrorLocation,
    },

    #[error("tantivy error during {context} at {location}")]
    Tantivy {
        context: String,
        #[source]
        source: Box<tantivy::TantivyError>,
        location: ErrorLocation,
    },

    #[error("tantivy query error during {context} at {location}")]
    Query {
        context: String,
        #[source]
        source: QueryParserError,
        location: ErrorLocation,
    },

    #[error("invalid tantivy index at {location}: {message}")]
    InvalidIndex {
        message: String,
        location: ErrorLocation,
    },
}

impl TantivySearchError {
    #[track_caller]
    pub fn io(context: impl Into<String>, path: Option<PathBuf>, source: io::Error) -> Self {
        Self::Io {
            context: context.into(),
            path,
            source,
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn tantivy(context: impl Into<String>, source: tantivy::TantivyError) -> Self {
        Self::Tantivy {
            context: context.into(),
            source: Box::new(source),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn query(context: impl Into<String>, source: QueryParserError) -> Self {
        Self::Query {
            context: context.into(),
            source,
            location: ErrorLocation::from(Location::caller()),
        }
    }

    #[track_caller]
    pub fn invalid_index(message: impl Into<String>) -> Self {
        Self::InvalidIndex {
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }
}
