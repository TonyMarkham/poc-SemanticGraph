use crate::{
    model::{GraphLanguage, ReferenceBatchExtraction},
    persist::{PersistenceRun, ScopedRoute},
};

use semantic_graph_db_manager::{DbWriteProgressCallback, WriteHandle};
use std::collections::HashMap;

pub(crate) struct ReferenceRouteWriteBatchRequest<'a> {
    pub(crate) store: &'a WriteHandle,
    pub(crate) run: PersistenceRun,
    pub(crate) route: ScopedRoute<'a>,
    pub(crate) extraction: &'a ReferenceBatchExtraction,
    pub(crate) file_ids: &'a HashMap<String, i64>,
    pub(crate) language: GraphLanguage,
    pub(crate) progress: Option<DbWriteProgressCallback>,
}
