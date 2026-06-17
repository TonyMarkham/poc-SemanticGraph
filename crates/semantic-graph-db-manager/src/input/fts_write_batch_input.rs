use crate::{FtsWriteBatchDocumentInput, FtsWriteBatchSeenDocumentInput};

#[derive(Debug, Clone, Default)]
pub struct FtsWriteBatchInput {
    pub documents: Vec<FtsWriteBatchDocumentInput>,
    pub seen_documents: Vec<FtsWriteBatchSeenDocumentInput>,
}
