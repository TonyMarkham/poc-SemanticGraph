use crate::{
    RustAnalyzerLibError, RustAnalyzerLibResult,
    model::{
        ResolvedCallTarget, ResolvedOutgoingCallSet, ResolvedReferenceSet, ResolvedReferenceTarget,
    },
    semantic::{
        DocumentSymbolItems, ProgressCallback, SharedAnalysisHost, SharedAnalysisWorker,
        SharedAnalysisWorkerHandle,
    },
};

use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
};
use tokio::task::JoinError;

#[derive(Clone)]
pub struct SharedAnalysisWorkerPool {
    workers: Arc<Vec<SharedAnalysisWorkerHandle>>,
    next_worker: Arc<AtomicUsize>,
}

impl SharedAnalysisWorkerPool {
    pub fn start(
        workspace_root: impl AsRef<Path>,
        worker_count: usize,
    ) -> RustAnalyzerLibResult<Self> {
        if worker_count == 0 {
            return Err(RustAnalyzerLibError::analysis_message(
                "start shared analysis worker pool",
                "shared analysis worker pool must contain at least one worker",
            ));
        }

        let host = SharedAnalysisHost::load(workspace_root.as_ref())?;
        let mut startup_handles = Vec::with_capacity(worker_count);
        for _ in 0..worker_count {
            let snapshot = host.snapshot();
            startup_handles.push(thread::spawn(move || SharedAnalysisWorker::start(snapshot)));
        }

        let mut workers = Vec::with_capacity(worker_count);
        let mut first_error = None;
        for startup_handle in startup_handles {
            match startup_handle.join() {
                Ok(Ok(worker)) => workers.push(worker),
                Ok(Err(error)) => {
                    if first_error.is_none() {
                        first_error = Some(error);
                    }
                }
                Err(_panic) => {
                    if first_error.is_none() {
                        first_error = Some(RustAnalyzerLibError::analysis_message(
                            "start shared analysis worker pool",
                            "shared analysis worker startup thread panicked",
                        ));
                    }
                }
            }
        }

        if let Some(error) = first_error {
            return Err(error);
        }

        Ok(Self {
            workers: Arc::new(workers),
            next_worker: Arc::new(AtomicUsize::new(0)),
        })
    }

    pub fn worker_count(&self) -> usize {
        self.workers.len()
    }

    pub fn worker_handles(&self) -> Vec<SharedAnalysisWorkerHandle> {
        self.workers.iter().cloned().collect()
    }

    pub async fn document_symbols_for_files(
        &self,
        file_paths: Vec<PathBuf>,
    ) -> RustAnalyzerLibResult<DocumentSymbolItems> {
        self.document_symbols_for_files_internal(file_paths, None)
            .await
    }

    pub async fn document_symbols_for_files_with_progress(
        &self,
        file_paths: Vec<PathBuf>,
        progress: ProgressCallback,
    ) -> RustAnalyzerLibResult<DocumentSymbolItems> {
        self.document_symbols_for_files_internal(file_paths, Some(progress))
            .await
    }

    async fn document_symbols_for_files_internal(
        &self,
        file_paths: Vec<PathBuf>,
        progress: Option<ProgressCallback>,
    ) -> RustAnalyzerLibResult<DocumentSymbolItems> {
        if file_paths.is_empty() {
            return Ok(Vec::new());
        }
        if self.workers.len() == 1 {
            return match progress {
                Some(progress) => {
                    self.workers[0]
                        .document_symbols_for_files_with_progress(file_paths, progress)
                        .await
                }
                None => self.workers[0].document_symbols_for_files(file_paths).await,
            };
        }

        let mut assignments = vec![Vec::new(); self.workers.len()];
        for (index, file_path) in file_paths.into_iter().enumerate() {
            assignments[index % self.workers.len()].push((index, file_path));
        }

        let mut handles = Vec::new();
        for (worker_index, assignment) in assignments.into_iter().enumerate() {
            if assignment.is_empty() {
                continue;
            }

            let worker = self.workers[worker_index].clone();
            let indices = assignment
                .iter()
                .map(|(index, _file_path)| *index)
                .collect::<Vec<_>>();
            let file_paths = assignment
                .into_iter()
                .map(|(_index, file_path)| file_path)
                .collect::<Vec<_>>();
            let progress = progress.clone();
            handles.push(tokio::spawn(async move {
                let items = match progress {
                    Some(progress) => {
                        worker
                            .document_symbols_for_files_with_progress(file_paths, progress)
                            .await?
                    }
                    None => worker.document_symbols_for_files(file_paths).await?,
                };
                Ok::<_, RustAnalyzerLibError>(
                    indices
                        .into_iter()
                        .zip(items)
                        .collect::<Vec<(usize, (PathBuf, Vec<lsp_types::DocumentSymbol>))>>(),
                )
            }));
        }

        let mut indexed_items = Vec::new();
        for handle in handles {
            let mut items = handle.await.map_err(document_symbol_join_error)??;
            indexed_items.append(&mut items);
        }
        indexed_items.sort_by_key(|(index, _item)| *index);

        Ok(indexed_items
            .into_iter()
            .map(|(_index, item)| item)
            .collect())
    }

    pub async fn references_for_symbol(
        &self,
        target: ResolvedReferenceTarget,
    ) -> RustAnalyzerLibResult<ResolvedReferenceSet> {
        self.next_worker()?.references_for_symbol(target).await
    }

    pub async fn outgoing_calls_for_symbol(
        &self,
        target: ResolvedCallTarget,
    ) -> RustAnalyzerLibResult<ResolvedOutgoingCallSet> {
        self.next_worker()?.outgoing_calls_for_symbol(target).await
    }

    pub async fn shutdown(&self) -> RustAnalyzerLibResult<()> {
        let mut handles = Vec::with_capacity(self.workers.len());
        for worker in self.workers.iter() {
            let worker = worker.clone();
            handles.push(tokio::spawn(async move { worker.shutdown().await }));
        }

        let mut first_error = None;
        for handle in handles {
            match handle.await {
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    if first_error.is_none() {
                        first_error = Some(error);
                    }
                }
                Err(error) => {
                    if first_error.is_none() {
                        first_error = Some(shutdown_join_error(error));
                    }
                }
            }
        }

        match first_error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    fn next_worker(&self) -> RustAnalyzerLibResult<SharedAnalysisWorkerHandle> {
        let index = self.next_worker.fetch_add(1, Ordering::Relaxed) % self.workers.len();
        self.workers.get(index).cloned().ok_or_else(|| {
            RustAnalyzerLibError::analysis_message(
                "select shared analysis worker",
                "shared analysis worker pool contained no workers",
            )
        })
    }
}

fn document_symbol_join_error(error: JoinError) -> RustAnalyzerLibError {
    RustAnalyzerLibError::analysis_message(
        "join shared analysis worker pool document symbol task",
        error.to_string(),
    )
}

fn shutdown_join_error(error: JoinError) -> RustAnalyzerLibError {
    RustAnalyzerLibError::analysis_message(
        "join shared analysis worker pool shutdown task",
        error.to_string(),
    )
}
