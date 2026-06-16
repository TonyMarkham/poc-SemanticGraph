use crate::workspace_all::WorkspaceExtractionRoutes;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadedWorkspaceAllConfig {
    reference_jobs: usize,
    call_jobs: usize,
    analysis_workers: usize,
    reference_analysis_workers: usize,
    call_analysis_workers: usize,
    routes: WorkspaceExtractionRoutes,
}

impl ThreadedWorkspaceAllConfig {
    pub fn new(
        reference_jobs: usize,
        call_jobs: usize,
        analysis_workers: usize,
        reference_analysis_workers: usize,
        call_analysis_workers: usize,
    ) -> Self {
        Self {
            reference_jobs,
            call_jobs,
            analysis_workers,
            reference_analysis_workers,
            call_analysis_workers,
            routes: WorkspaceExtractionRoutes::all(),
        }
    }

    pub fn with_routes(
        reference_jobs: usize,
        call_jobs: usize,
        analysis_workers: usize,
        reference_analysis_workers: usize,
        call_analysis_workers: usize,
        routes: WorkspaceExtractionRoutes,
    ) -> Self {
        Self {
            reference_jobs,
            call_jobs,
            analysis_workers,
            reference_analysis_workers,
            call_analysis_workers,
            routes,
        }
    }

    pub fn reference_jobs(&self) -> usize {
        self.reference_jobs
    }

    pub fn call_jobs(&self) -> usize {
        self.call_jobs
    }

    pub fn analysis_workers(&self) -> usize {
        self.analysis_workers
    }

    pub fn reference_analysis_workers(&self) -> usize {
        self.reference_analysis_workers
    }

    pub fn call_analysis_workers(&self) -> usize {
        self.call_analysis_workers
    }

    pub fn routes(&self) -> WorkspaceExtractionRoutes {
        self.routes
    }
}
