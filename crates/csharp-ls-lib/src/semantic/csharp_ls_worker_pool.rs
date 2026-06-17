use crate::{CSharpLsLibError, CSharpLsLibResult, model::FileSemanticWork};

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::semantic::CSharpLsWorker;

pub struct CSharpLsWorkerPool {
    workers: Vec<CSharpLsWorker>,
    opened_file_workers: HashMap<PathBuf, usize>,
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

        Ok(Self {
            workers,
            opened_file_workers: HashMap::new(),
        })
    }

    pub fn worker_count(&self) -> usize {
        self.workers.len()
    }

    pub async fn document_symbols_for_files(
        &mut self,
        file_paths: Vec<PathBuf>,
    ) -> CSharpLsLibResult<Vec<(PathBuf, Vec<lsp_types::DocumentSymbol>)>> {
        let worker_index = 0;
        let worker = self.workers.get_mut(worker_index).ok_or_else(|| {
            CSharpLsLibError::response_shape(
                "document_symbols_for_files",
                "worker pool contained no workers",
            )
        })?;
        let results = worker
            .document_symbols_for_files(file_paths.clone())
            .await?;
        for file_path in file_paths {
            self.opened_file_workers.insert(file_path, worker_index);
        }

        Ok(results)
    }

    pub async fn file_semantic_work_items(
        &mut self,
        work_items: Vec<FileSemanticWork>,
    ) -> CSharpLsLibResult<Vec<crate::model::FileSemanticResult>> {
        let mut results = Vec::with_capacity(work_items.len());
        for (index, work) in work_items.into_iter().enumerate() {
            let worker_index = self.worker_index_for_file(&work.file_path, index);
            let file_path = work.file_path.clone();
            results.push(self.workers[worker_index].file_semantic_work(work).await?);
            self.opened_file_workers.insert(file_path, worker_index);
        }

        Ok(results)
    }

    fn worker_index_for_file(&self, file_path: &Path, fallback_index: usize) -> usize {
        self.opened_file_workers
            .get(file_path)
            .copied()
            .unwrap_or(fallback_index % self.workers.len())
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
