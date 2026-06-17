#[derive(Debug, Clone)]
pub struct CloseStaleFtsDocumentsInput<'a> {
    pub workspace_id: i64,
    pub run_id: i64,
    pub provider: &'a str,
    pub route: &'a str,
    pub scope: &'a str,
    pub scope_key: &'a str,
}
