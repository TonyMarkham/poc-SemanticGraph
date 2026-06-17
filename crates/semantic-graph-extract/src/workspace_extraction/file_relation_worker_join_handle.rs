use crate::{ExtractResult, workspace_extraction::FileRelationWorkerResult};

pub(crate) type FileRelationWorkerJoinHandle =
    tokio::task::JoinHandle<ExtractResult<FileRelationWorkerResult>>;
