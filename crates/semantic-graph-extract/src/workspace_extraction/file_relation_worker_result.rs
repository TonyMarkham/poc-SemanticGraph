use crate::workspace_extraction::FileRelationWorkerMetric;

pub(crate) type FileRelationWorkerResult = (
    Vec<rust_analyzer_lib::FileSemanticResult>,
    FileRelationWorkerMetric,
);
