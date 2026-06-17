use crate::TantivyFtsDocument;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TantivyFtsIndexUpdate {
    pub documents: Vec<TantivyFtsDocument>,
    pub deleted_uris: Vec<String>,
    pub indexing_workers: usize,
}
