use crate::model::RouteScope;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ScopedRoute<'a> {
    pub(crate) scope: &'a str,
    pub(crate) scope_key: &'a str,
}

impl<'a> ScopedRoute<'a> {
    pub(crate) fn file(scope_key: &'a str) -> Self {
        Self {
            scope: RouteScope::FILE.as_str(),
            scope_key,
        }
    }

    pub(crate) fn workspace(scope_key: &'a str) -> Self {
        Self {
            scope: RouteScope::WORKSPACE.as_str(),
            scope_key,
        }
    }
}
