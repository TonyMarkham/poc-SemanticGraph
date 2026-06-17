use crate::{
    TantivyFtsDocument, TantivyFtsFields, TantivyFtsIndexUpdate, TantivyFtsIndexUpdateSummary,
    TantivySearchError, TantivySearchResult,
};

use std::{fs, path::Path};
use tantivy::{
    Index, TantivyDocument, Term,
    collector::Count,
    doc,
    query::QueryParser,
    schema::{IndexRecordOption, STORED, STRING, Schema, TextFieldIndexing, TextOptions},
    tokenizer::{LowerCaser, NgramTokenizer, TextAnalyzer},
};

const CI_NGRAM_TOKENIZER: &str = "semantic_graph_ngram_ci";
const CS_NGRAM_TOKENIZER: &str = "semantic_graph_ngram_cs";
const NGRAM_SIZE: usize = 3;
const MEMORY_BUDGET_PER_WORKER_BYTES: usize = 32 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct TantivyFtsIndex {
    index: Index,
    fields: TantivyFtsFields,
}

impl TantivyFtsIndex {
    pub fn open_or_create(index_path: &Path) -> TantivySearchResult<Self> {
        fs::create_dir_all(index_path).map_err(|source| {
            TantivySearchError::io(
                "create tantivy index directory",
                Some(index_path.into()),
                source,
            )
        })?;

        let index = if index_path.join("meta.json").exists() {
            Index::open_in_dir(index_path)
                .map_err(|source| TantivySearchError::tantivy("open tantivy index", source))?
        } else {
            Index::create_in_dir(index_path, fts_schema())
                .map_err(|source| TantivySearchError::tantivy("create tantivy index", source))?
        };
        register_tokenizers(&index)?;
        let fields = TantivyFtsFields::from_schema(&index.schema())?;

        Ok(Self { index, fields })
    }

    pub fn apply_update(
        &self,
        update: TantivyFtsIndexUpdate,
    ) -> TantivySearchResult<TantivyFtsIndexUpdateSummary> {
        let indexing_workers = update.indexing_workers.max(1);
        let has_updates = !update.documents.is_empty() || !update.deleted_uris.is_empty();
        let memory_budget_bytes = indexing_workers.saturating_mul(MEMORY_BUDGET_PER_WORKER_BYTES);
        if !has_updates {
            return Ok(TantivyFtsIndexUpdateSummary {
                indexing_workers,
                memory_budget_bytes,
                ..TantivyFtsIndexUpdateSummary::default()
            });
        }

        let mut writer = self
            .index
            .writer_with_num_threads::<TantivyDocument>(indexing_workers, memory_budget_bytes)
            .map_err(|source| TantivySearchError::tantivy("open tantivy index writer", source))?;
        for uri in &update.deleted_uris {
            writer.delete_term(Term::from_field_text(self.fields.uri, uri));
        }
        for document in &update.documents {
            writer.delete_term(Term::from_field_text(self.fields.uri, &document.uri));
            writer
                .add_document(tantivy_document(self.fields, document))
                .map_err(|source| TantivySearchError::tantivy("add tantivy document", source))?;
        }
        writer
            .commit()
            .map_err(|source| TantivySearchError::tantivy("commit tantivy index", source))?;

        Ok(TantivyFtsIndexUpdateSummary {
            indexed_documents: update.documents.len(),
            deleted_uris: update.deleted_uris.len(),
            committed: true,
            indexing_workers,
            memory_budget_bytes,
        })
    }

    pub fn count_case_insensitive_candidates(&self, query: &str) -> TantivySearchResult<usize> {
        let reader = self
            .index
            .reader()
            .map_err(|source| TantivySearchError::tantivy("open tantivy index reader", source))?;
        let searcher = reader.searcher();
        let query_parser = QueryParser::for_index(&self.index, vec![self.fields.content_ci]);
        let query = query_parser
            .parse_query(query)
            .map_err(|source| TantivySearchError::query("parse tantivy query", source))?;
        searcher
            .search(&query, &Count)
            .map_err(|source| TantivySearchError::tantivy("search tantivy index", source))
    }
}

fn tantivy_document(fields: TantivyFtsFields, document: &TantivyFtsDocument) -> TantivyDocument {
    doc!(
        fields.uri => document.uri.clone(),
        fields.path => document.path.clone(),
        fields.language => document.language.clone(),
        fields.content_hash => document.content_hash.clone(),
        fields.content_ci => document.content.clone(),
        fields.content_cs => document.content.clone()
    )
}

fn fts_schema() -> Schema {
    let mut schema_builder = Schema::builder();
    schema_builder.add_text_field("uri", STRING | STORED);
    schema_builder.add_text_field("path", STRING | STORED);
    schema_builder.add_text_field("language", STRING | STORED);
    schema_builder.add_text_field("content_hash", STRING | STORED);
    schema_builder.add_text_field("content_ci", ngram_text_options(CI_NGRAM_TOKENIZER));
    schema_builder.add_text_field("content_cs", ngram_text_options(CS_NGRAM_TOKENIZER));
    schema_builder.build()
}

fn ngram_text_options(tokenizer_name: &str) -> TextOptions {
    let indexing = TextFieldIndexing::default()
        .set_tokenizer(tokenizer_name)
        .set_index_option(IndexRecordOption::WithFreqsAndPositions);
    TextOptions::default().set_indexing_options(indexing)
}

fn register_tokenizers(index: &Index) -> TantivySearchResult<()> {
    let ci_ngram = NgramTokenizer::new(NGRAM_SIZE, NGRAM_SIZE, false).map_err(|source| {
        TantivySearchError::tantivy("create lowercase ngram tokenizer", source)
    })?;
    let cs_ngram = NgramTokenizer::new(NGRAM_SIZE, NGRAM_SIZE, false).map_err(|source| {
        TantivySearchError::tantivy("create case-sensitive ngram tokenizer", source)
    })?;
    let ci_tokenizer = TextAnalyzer::builder(ci_ngram).filter(LowerCaser).build();
    let cs_tokenizer = TextAnalyzer::builder(cs_ngram).build();
    index
        .tokenizers()
        .register(CI_NGRAM_TOKENIZER, ci_tokenizer);
    index
        .tokenizers()
        .register(CS_NGRAM_TOKENIZER, cs_tokenizer);
    Ok(())
}
