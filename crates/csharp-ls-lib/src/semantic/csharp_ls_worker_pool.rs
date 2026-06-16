use crate::{CSharpLsLibError, CSharpLsLibResult, model::FileSemanticWork};

use std::path::PathBuf;

use crate::semantic::CSharpLsWorker;

pub struct CSharpLsWorkerPool {
    workers: Vec<CSharpLsWorker>,
}

impl CSharpLsWorkerPool {
    pub async fn start(
        binary: PathBuf,
        solution: PathBuf,
        log_level: String,
        features: Vec<String>,
        startup_timeout_ms: u64,
        request_timeout_ms: u64,
        worker_count: usize,
    ) -> CSharpLsLibResult<Self> {
        if worker_count == 0 {
            return Err(CSharpLsLibError::response_shape(
                "CSharpLsWorkerPool::start",
                "worker_count must be greater than zero",
            ));
        }

        let mut workers = Vec::with_capacity(worker_count);
        for _ in 0..worker_count {
            workers.push(
                CSharpLsWorker::start(
                    binary.clone(),
                    solution.clone(),
                    log_level.clone(),
                    features.clone(),
                    startup_timeout_ms,
                    request_timeout_ms,
                )
                .await?,
            );
        }

        Ok(Self { workers })
    }

    pub fn worker_count(&self) -> usize {
        self.workers.len()
    }

    pub async fn document_symbols_for_files(
        &mut self,
        file_paths: Vec<PathBuf>,
    ) -> CSharpLsLibResult<Vec<(PathBuf, Vec<lsp_types::DocumentSymbol>)>> {
        let worker = self.workers.first_mut().ok_or_else(|| {
            CSharpLsLibError::response_shape(
                "document_symbols_for_files",
                "worker pool contained no workers",
            )
        })?;
        worker.document_symbols_for_files(file_paths).await
    }

    pub async fn file_semantic_work_items(
        &mut self,
        work_items: Vec<FileSemanticWork>,
    ) -> CSharpLsLibResult<Vec<crate::model::FileSemanticResult>> {
        let mut results = Vec::with_capacity(work_items.len());
        for (index, work) in work_items.into_iter().enumerate() {
            let worker_index = index % self.workers.len();
            results.push(self.workers[worker_index].file_semantic_work(work).await?);
        }

        Ok(results)
    }

    pub async fn shutdown(self) -> CSharpLsLibResult<()> {
        let mut first_error = None;
        for worker in self.workers {
            if let Err(error) = worker.shutdown().await
                && first_error.is_none()
            {
                first_error = Some(error);
            }
        }

        match first_error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}
