use std::future::Future;

use crate::error::ExtractError;
use crate::model::{DocumentSymbolExtraction, DocumentSymbolRequest, GraphLanguage, ProviderId};

pub trait DocumentSymbolProvider {
    fn provider_id(&self) -> ProviderId;

    fn language(&self) -> GraphLanguage;

    fn extract_document_symbols(
        &self,
        request: DocumentSymbolRequest,
    ) -> impl Future<Output = Result<DocumentSymbolExtraction, ExtractError>> + Send;
}
