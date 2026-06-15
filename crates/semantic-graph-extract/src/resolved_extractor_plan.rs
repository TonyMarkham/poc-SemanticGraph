use semantic_graph_config::ExtractorMode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedExtractorPlan {
    pub mode: ExtractorMode,
    pub reference_jobs: usize,
    pub call_jobs: usize,
    pub analysis_workers: usize,
    pub reference_analysis_workers: usize,
    pub call_analysis_workers: usize,
}
