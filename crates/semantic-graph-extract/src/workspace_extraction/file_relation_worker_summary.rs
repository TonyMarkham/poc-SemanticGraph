use crate::{
    model::{CallRouteSummary, ReferenceRouteSummary},
    persist::PersistenceSummary,
};

pub(crate) struct FileRelationWorkerSummary {
    pub(crate) reference_persistence: PersistenceSummary,
    pub(crate) reference_route: ReferenceRouteSummary,
    pub(crate) call_persistence: PersistenceSummary,
    pub(crate) call_route: CallRouteSummary,
}
