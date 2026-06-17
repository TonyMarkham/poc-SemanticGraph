use crate::{TantivySearchError, TantivySearchResult};

use tantivy::schema::{Field, Schema};

#[derive(Debug, Clone, Copy)]
pub(crate) struct TantivyFtsFields {
    pub(crate) uri: Field,
    pub(crate) path: Field,
    pub(crate) language: Field,
    pub(crate) content_hash: Field,
    pub(crate) content_ci: Field,
    pub(crate) content_cs: Field,
}

impl TantivyFtsFields {
    pub(crate) fn from_schema(schema: &Schema) -> TantivySearchResult<Self> {
        Ok(Self {
            uri: field(schema, "uri")?,
            path: field(schema, "path")?,
            language: field(schema, "language")?,
            content_hash: field(schema, "content_hash")?,
            content_ci: field(schema, "content_ci")?,
            content_cs: field(schema, "content_cs")?,
        })
    }
}

fn field(schema: &Schema, name: &str) -> TantivySearchResult<Field> {
    schema.get_field(name).map_err(|error| {
        TantivySearchError::invalid_index(format!("missing expected field {name}: {error}"))
    })
}
