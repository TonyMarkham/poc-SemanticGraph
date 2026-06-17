#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FtsExtractionOptions {
    no_rust: bool,
    no_csharp: bool,
    no_submodules: bool,
}

impl FtsExtractionOptions {
    pub fn new(no_rust: bool, no_csharp: bool, no_submodules: bool) -> Self {
        Self {
            no_rust,
            no_csharp,
            no_submodules,
        }
    }

    pub fn no_rust(self) -> bool {
        self.no_rust
    }

    pub fn no_csharp(self) -> bool {
        self.no_csharp
    }

    pub fn no_submodules(self) -> bool {
        self.no_submodules
    }
}
