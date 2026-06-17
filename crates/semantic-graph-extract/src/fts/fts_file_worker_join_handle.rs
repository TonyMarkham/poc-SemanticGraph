use crate::{ExtractResult, fts::FtsFileWorkResult, fts::FtsFileWorkerMetric};

pub(crate) type FtsFileWorkerJoinHandle =
    tokio::task::JoinHandle<ExtractResult<(Vec<FtsFileWorkResult>, FtsFileWorkerMetric)>>;
