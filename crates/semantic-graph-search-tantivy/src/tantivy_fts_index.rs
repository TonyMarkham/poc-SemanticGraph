use crate::{
    TantivyFtsDocument, TantivyFtsFields, TantivyFtsIndexUpdate, TantivyFtsIndexUpdateSummary,
    TantivyFtsSearchHit, TantivyFtsSearchRequest, TantivyFtsSearchResults, TantivySearchError,
    TantivySearchResult,
};

use std::{fs, path::Path};
use tantivy::{
    Index, Order, TantivyDocument, Term,
    collector::{
        Count, TopDocs,
        sort_key::{SortBySimilarityScore, SortByString},
    },
    doc,
    query::{BooleanQuery, ConstScoreQuery, Occur, Query, QueryParser, RegexQuery, TermQuery},
    schema::{
        FAST, IndexRecordOption, STORED, STRING, Schema, TextFieldIndexing, TextOptions, Value,
    },
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

    pub fn open_read_only(index_path: &Path) -> TantivySearchResult<Self> {
        if !index_path.join("meta.json").is_file() {
            return Err(TantivySearchError::invalid_index(format!(
                "tantivy index not found at {}",
                index_path.display()
            )));
        }

        let index = Index::open_in_dir(index_path)
            .map_err(|source| TantivySearchError::tantivy("open tantivy index", source))?;
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

    pub fn search(
        &self,
        request: TantivyFtsSearchRequest,
    ) -> TantivySearchResult<TantivyFtsSearchResults> {
        let limit_with_next = request.limit.saturating_add(1);
        let reader = self
            .index
            .reader()
            .map_err(|source| TantivySearchError::tantivy("open tantivy index reader", source))?;
        let searcher = reader.searcher();
        let query = self.search_query(&request)?;
        let top_docs = searcher
            .search(
                &query,
                &TopDocs::with_limit(limit_with_next)
                    .and_offset(request.offset)
                    .order_by((
                        (SortBySimilarityScore, Order::Desc),
                        (SortByString::for_field("uri"), Order::Asc),
                        (SortByString::for_field("content_hash"), Order::Asc),
                    )),
            )
            .map_err(|source| TantivySearchError::tantivy("search tantivy index", source))?;

        let has_more = top_docs.len() > request.limit;
        let hits = top_docs
            .into_iter()
            .take(request.limit)
            .map(|((score, _uri_sort, _content_hash_sort), address)| {
                let document = searcher.doc::<TantivyDocument>(address).map_err(|source| {
                    TantivySearchError::tantivy("load tantivy stored document", source)
                })?;
                Ok(TantivyFtsSearchHit {
                    uri: stored_text(&document, self.fields.uri, "uri")?,
                    path: stored_text(&document, self.fields.path, "path")?,
                    language: stored_text(&document, self.fields.language, "language")?,
                    content_hash: stored_text(&document, self.fields.content_hash, "content_hash")?,
                    score,
                })
            })
            .collect::<TantivySearchResult<Vec<_>>>()?;

        Ok(TantivyFtsSearchResults { hits, has_more })
    }

    fn search_query(
        &self,
        request: &TantivyFtsSearchRequest,
    ) -> TantivySearchResult<Box<dyn Query>> {
        let content_field = if request.case_sensitive {
            self.fields.content_cs
        } else {
            self.fields.content_ci
        };
        let query_parser = QueryParser::for_index(&self.index, vec![content_field]);
        let content_query = query_parser
            .parse_query(&request.query)
            .map_err(|source| TantivySearchError::query("parse tantivy query", source))?;

        let mut queries = vec![(Occur::Must, content_query)];
        if let Some(language) = &request.language {
            let language_query = TermQuery::new(
                Term::from_field_text(self.fields.language, language),
                IndexRecordOption::Basic,
            );
            queries.push((
                Occur::Must,
                Box::new(ConstScoreQuery::new(Box::new(language_query), 0.0)),
            ));
        }
        if let Some(path_prefix) = &request.path_prefix {
            let path_query = RegexQuery::from_pattern(
                &format!("{}.*", escape_regex(path_prefix)),
                self.fields.path,
            )
            .map_err(|source| TantivySearchError::tantivy("build path prefix query", source))?;
            queries.push((
                Occur::Must,
                Box::new(ConstScoreQuery::new(Box::new(path_query), 0.0)),
            ));
        }

        if queries.len() == 1 {
            let (_occur, query) = queries.remove(0);
            return Ok(query);
        }

        Ok(Box::new(BooleanQuery::new(queries)))
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
    schema_builder.add_text_field("uri", STRING | STORED | FAST);
    schema_builder.add_text_field("path", STRING | STORED);
    schema_builder.add_text_field("language", STRING | STORED);
    schema_builder.add_text_field("content_hash", STRING | STORED | FAST);
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

fn stored_text(
    document: &TantivyDocument,
    field: tantivy::schema::Field,
    name: &str,
) -> TantivySearchResult<String> {
    let value = document
        .get_first(field)
        .and_then(|value| value.as_str())
        .ok_or_else(|| TantivySearchError::invalid_index(format!("missing stored field {name}")))?;

    Ok(value.to_string())
}

fn escape_regex(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        if matches!(
            character,
            '.' | '+' | '*' | '?' | '^' | '$' | '(' | ')' | '[' | ']' | '{' | '}' | '|' | '\\'
        ) {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
}
