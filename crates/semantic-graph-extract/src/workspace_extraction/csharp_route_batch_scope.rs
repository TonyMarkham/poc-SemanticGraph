#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CSharpRouteBatchScope {
    Project,
    Solution,
}

impl CSharpRouteBatchScope {
    pub fn benchmark_prefix(self) -> &'static str {
        match self {
            Self::Project => "csharp_project",
            Self::Solution => "csharp_solution",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Project => "project",
            Self::Solution => "solution",
        }
    }
}
