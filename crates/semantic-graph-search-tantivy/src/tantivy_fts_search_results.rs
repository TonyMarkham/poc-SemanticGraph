use crate::TantivyFtsSearchHit;

#[derive(Debug, Clone, PartialEq)]
pub struct TantivyFtsSearchResults {
    pub hits: Vec<TantivyFtsSearchHit>,
    pub has_more: bool,
}
