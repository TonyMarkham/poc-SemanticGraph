use crate::{
    ExtractError,
    model::{DocumentSymbolExtraction, DocumentSymbolRequest, GraphLanguage, ProviderId},
};

use std::future::Future;

pub trait DocumentSymbolProvider {
    fn provider_id(&self) -> ProviderId;

    fn language(&self) -> GraphLanguage;

    fn extract_document_symbols(
        &self,
        request: DocumentSymbolRequest,
    ) -> impl Future<Output = Result<DocumentSymbolExtraction, ExtractError>> + Send;
}
