#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RouteStatusFilters<'a> {
    pub(crate) workspace_id: Option<i64>,
    pub(crate) root_uri: Option<&'a str>,
    pub(crate) route: Option<&'a str>,
    pub(crate) scope: Option<&'a str>,
    pub(crate) scope_key: Option<&'a str>,
    pub(crate) file_path: Option<&'a str>,
    pub(crate) limit: i64,
}
