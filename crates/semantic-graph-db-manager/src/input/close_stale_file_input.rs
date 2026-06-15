#[derive(Debug, Clone)]
pub struct CloseStaleFileInput<'a> {
    pub workspace_id: i64,
    pub run_id: i64,
    pub file_uri: &'a str,
}
