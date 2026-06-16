use crate::{
    resources::{
        local_testbeds_resource::{LOCAL_TESTBEDS_RESOURCE_URI, local_testbeds_resource_text},
        routes_resource::{ROUTES_RESOURCE_URI, routes_resource_text},
        schema_resource::{SCHEMA_RESOURCE_URI, schema_resource_text},
        workspace_resource::{WORKSPACE_RESOURCE_URI, workspace_resource_text},
    },
    sanitize::{DEFAULT_TEXT_CAP, sanitize_transcript_text},
    server::ServerState,
};

use rmcp::{
    ErrorData,
    model::{AnnotateAble, RawResource, ReadResourceResult, Resource, ResourceContents},
};
use serde_json::json;

pub struct ResourceRegistry;

impl ResourceRegistry {
    #[cfg(test)]
    pub fn resource_uris() -> Vec<&'static str> {
        vec![
            SCHEMA_RESOURCE_URI,
            WORKSPACE_RESOURCE_URI,
            ROUTES_RESOURCE_URI,
            LOCAL_TESTBEDS_RESOURCE_URI,
        ]
    }

    pub fn resources() -> Vec<Resource> {
        vec![
            resource(
                SCHEMA_RESOURCE_URI,
                "schema",
                "Compact SemanticGraph SQLite schema summary.",
            ),
            resource(
                WORKSPACE_RESOURCE_URI,
                "workspace",
                "Current read-only server context and latest extraction run summaries.",
            ),
            resource(
                ROUTES_RESOURCE_URI,
                "routes",
                "Extractor route names and freshness semantics.",
            ),
            resource(
                LOCAL_TESTBEDS_RESOURCE_URI,
                "local-testbeds",
                "Local visualization testbed notes.",
            ),
        ]
    }

    pub async fn read_resource(
        state: &ServerState,
        uri: &str,
    ) -> Result<ReadResourceResult, ErrorData> {
        let text = match uri {
            SCHEMA_RESOURCE_URI => schema_resource_text(),
            WORKSPACE_RESOURCE_URI => workspace_resource_text(state).await?,
            ROUTES_RESOURCE_URI => routes_resource_text(),
            LOCAL_TESTBEDS_RESOURCE_URI => local_testbeds_resource_text(),
            _ => {
                return Err(ErrorData::resource_not_found(
                    "resource not found",
                    Some(json!({ "uri": sanitize_transcript_text(uri, DEFAULT_TEXT_CAP) })),
                ));
            }
        };

        Ok(ReadResourceResult::new(vec![
            ResourceContents::text(
                sanitize_transcript_text(&text, DEFAULT_TEXT_CAP * 16),
                uri.to_string(),
            )
            .with_mime_type("text/plain"),
        ]))
    }
}

fn resource(uri: &'static str, name: &'static str, description: &'static str) -> Resource {
    RawResource::new(uri, name)
        .with_description(description)
        .with_mime_type("text/plain")
        .no_annotation()
}

#[cfg(test)]
mod tests {
    use crate::resources::ResourceRegistry;

    #[test]
    fn lists_every_phase_two_resource_once() {
        assert_eq!(
            vec![
                "semantic-graph://schema",
                "semantic-graph://workspace",
                "semantic-graph://routes",
                "semantic-graph://local-testbeds",
            ],
            ResourceRegistry::resource_uris()
        );

        assert_eq!(
            ResourceRegistry::resource_uris().len(),
            ResourceRegistry::resources().len()
        );
    }
}
