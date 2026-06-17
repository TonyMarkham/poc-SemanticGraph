use crate::workspace_extraction::CSharpRouteBatchScope;

use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CSharpRouteBatchContext {
    scope: CSharpRouteBatchScope,
    discovery_elapsed: Option<Duration>,
}

impl CSharpRouteBatchContext {
    pub fn new(scope: CSharpRouteBatchScope, discovery_elapsed: Option<Duration>) -> Self {
        Self {
            scope,
            discovery_elapsed,
        }
    }

    pub fn discovery_elapsed(self) -> Option<Duration> {
        self.discovery_elapsed
    }

    pub fn scope(self) -> CSharpRouteBatchScope {
        self.scope
    }
}
