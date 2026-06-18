use crate::{
    QueryError, QueryResult,
    model::{FtsLineRange, FtsSearchHit, FtsSearchRequest, FtsSearchResults, FtsSearchSnippet},
    row::FtsDocumentContentRow,
    sqlite::open_read_only_pool,
};

use semantic_graph_config::QueryServiceConfig;
use semantic_graph_search_tantivy::{
    TantivyFtsIndex, TantivyFtsSearchHit, TantivyFtsSearchRequest,
};
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};

const DEFAULT_FTS_SEARCH_LIMIT: i64 = 25;
const DEFAULT_CONTEXT_LINES: i64 = 2;
const MAX_CONTEXT_LINES: i64 = 20;
const MAX_SNIPPETS_PER_HIT: usize = 3;

#[derive(Debug, Clone)]
pub struct FtsQueryService {
    fts_database_path: PathBuf,
    tantivy_index_path: PathBuf,
    query_service_config: QueryServiceConfig,
}

impl FtsQueryService {
    pub fn new(fts_database_path: PathBuf, tantivy_index_path: PathBuf) -> Self {
        Self::with_query_service_config(
            fts_database_path,
            tantivy_index_path,
            QueryServiceConfig::default(),
        )
    }

    pub fn with_query_service_config(
        fts_database_path: PathBuf,
        tantivy_index_path: PathBuf,
        query_service_config: QueryServiceConfig,
    ) -> Self {
        Self {
            fts_database_path,
            tantivy_index_path,
            query_service_config,
        }
    }

    pub async fn search(&self, request: FtsSearchRequest) -> QueryResult<FtsSearchResults> {
        ensure_fts_database_exists(&self.fts_database_path)?;
        ensure_tantivy_index_exists(&self.tantivy_index_path)?;

        let query = required_text(request.query, "query")?;
        let language = optional_text(request.language, "language")?;
        let path_prefix = optional_text(request.path_prefix, "pathPrefix")?;
        let limit = resolve_limit(
            request.limit,
            DEFAULT_FTS_SEARCH_LIMIT,
            self.query_service_config.max_search_limit(),
            "limit",
        )?;
        let context_lines = resolve_context_lines(
            request.context_lines,
            DEFAULT_CONTEXT_LINES,
            MAX_CONTEXT_LINES,
        )?;
        let offset = resolve_cursor(request.cursor.as_deref())?;

        let index = TantivyFtsIndex::open_read_only(&self.tantivy_index_path)
            .map_err(QueryError::tantivy_search)?;
        let tantivy_results = index
            .search(TantivyFtsSearchRequest {
                query: query.clone(),
                limit: limit as usize,
                offset,
                language,
                path_prefix,
                case_sensitive: request.case_sensitive.unwrap_or(false),
            })
            .map_err(QueryError::tantivy_search)?;

        let pool = open_read_only_pool(&self.fts_database_path).await?;
        let mut hits = Vec::with_capacity(tantivy_results.hits.len());
        for hit in tantivy_results.hits {
            hits.push(
                hydrate_search_hit(
                    &pool,
                    hit,
                    &query,
                    request.case_sensitive.unwrap_or(false),
                    context_lines as usize,
                )
                .await?,
            );
        }

        Ok(FtsSearchResults {
            requested_limit: request.limit,
            applied_limit: limit,
            fts_database_path: self.fts_database_path.display().to_string(),
            tantivy_index_path: self.tantivy_index_path.display().to_string(),
            hits,
            next_cursor: tantivy_results
                .has_more
                .then(|| offset.saturating_add(limit as usize).to_string()),
        })
    }
}

async fn hydrate_search_hit(
    pool: &SqlitePool,
    hit: TantivyFtsSearchHit,
    query: &str,
    case_sensitive: bool,
    context_lines: usize,
) -> QueryResult<FtsSearchHit> {
    let content = load_fts_document_content(pool, &hit.uri, &hit.content_hash).await?;
    let snippets = snippets_for_content(&content, query, case_sensitive, context_lines);
    let line_range = line_range_for_snippets(&snippets);

    Ok(FtsSearchHit {
        uri: hit.uri,
        path: hit.path,
        language: hit.language,
        content_hash: hit.content_hash,
        score: hit.score,
        line_range,
        snippets,
    })
}

async fn load_fts_document_content(
    pool: &SqlitePool,
    uri: &str,
    content_hash: &str,
) -> QueryResult<String> {
    let row = sqlx::query_as::<_, FtsDocumentContentRow>(
        r#"
        SELECT content.content AS content
        FROM fts_documents document
        JOIN files file
          ON file.id = document.file_id
         AND file.workspace_id = document.workspace_id
        JOIN fts_document_contents content
          ON content.document_id = document.id
        WHERE file.uri = ?
          AND document.content_hash = ?
          AND document.valid_to_run_id IS NULL
        LIMIT 1
        "#,
    )
    .bind(uri)
    .bind(content_hash)
    .fetch_optional(pool)
    .await
    .map_err(QueryError::database)?;

    row.map(|row| row.content).ok_or_else(|| {
        QueryError::fts_consistency(format!(
            "tantivy hit uri '{uri}' content hash '{content_hash}' could not be hydrated from SQLite"
        ))
    })
}

fn snippets_for_content(
    content: &str,
    query: &str,
    case_sensitive: bool,
    context_lines: usize,
) -> Vec<FtsSearchSnippet> {
    let lines = content.lines().map(str::to_string).collect::<Vec<_>>();
    if lines.is_empty() {
        return vec![FtsSearchSnippet {
            line_range: FtsLineRange {
                start_line: 1,
                end_line: 1,
            },
            text: String::new(),
        }];
    }

    let match_indexes = matching_line_indexes(&lines, query, case_sensitive);
    let selected_indexes = if match_indexes.is_empty() {
        vec![0]
    } else {
        match_indexes
    };
    let mut windows = Vec::<(usize, usize)>::new();

    for line_index in selected_indexes.into_iter().take(MAX_SNIPPETS_PER_HIT) {
        let start = line_index.saturating_sub(context_lines);
        let end = line_index
            .saturating_add(context_lines)
            .min(lines.len().saturating_sub(1));

        match windows.last_mut() {
            Some((_last_start, last_end)) if start <= last_end.saturating_add(1) => {
                *last_end = (*last_end).max(end);
            }
            _ => windows.push((start, end)),
        }
    }

    windows
        .into_iter()
        .map(|(start, end)| FtsSearchSnippet {
            line_range: FtsLineRange {
                start_line: (start + 1) as i64,
                end_line: (end + 1) as i64,
            },
            text: lines[start..=end].join("\n"),
        })
        .collect()
}

fn matching_line_indexes(lines: &[String], query: &str, case_sensitive: bool) -> Vec<usize> {
    if case_sensitive {
        return lines
            .iter()
            .enumerate()
            .filter_map(|(index, line)| line.contains(query).then_some(index))
            .collect();
    }

    let query = query.to_lowercase();
    lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| line.to_lowercase().contains(&query).then_some(index))
        .collect()
}

fn line_range_for_snippets(snippets: &[FtsSearchSnippet]) -> FtsLineRange {
    let start_line = snippets
        .first()
        .map(|snippet| snippet.line_range.start_line)
        .unwrap_or(1);
    let end_line = snippets
        .last()
        .map(|snippet| snippet.line_range.end_line)
        .unwrap_or(start_line);

    FtsLineRange {
        start_line,
        end_line,
    }
}

fn ensure_fts_database_exists(path: &Path) -> QueryResult<()> {
    if path.is_file() {
        return Ok(());
    }

    Err(QueryError::setup(format!(
        "FTS SQLite database not found at {}. Run semantic-graph-extract fts first or pass --fts-database-path.",
        path.display()
    )))
}

fn ensure_tantivy_index_exists(path: &Path) -> QueryResult<()> {
    if path.join("meta.json").is_file() {
        return Ok(());
    }

    Err(QueryError::setup(format!(
        "FTS Tantivy index not found at {}. Run semantic-graph-extract fts first or pass --fts-database-path.",
        path.display()
    )))
}

fn resolve_limit(
    requested: Option<i64>,
    default_value: i64,
    maximum: i64,
    field_name: &str,
) -> QueryResult<i64> {
    let limit = requested.unwrap_or(default_value);

    if !(1..=maximum).contains(&limit) {
        return Err(QueryError::invalid_params(format!(
            "{field_name} must be between 1 and {maximum}"
        )));
    }

    Ok(limit)
}

fn resolve_context_lines(
    requested: Option<i64>,
    default_value: i64,
    maximum: i64,
) -> QueryResult<i64> {
    let context_lines = requested.unwrap_or(default_value);

    if !(0..=maximum).contains(&context_lines) {
        return Err(QueryError::invalid_params(format!(
            "contextLines must be between 0 and {maximum}"
        )));
    }

    Ok(context_lines)
}

fn resolve_cursor(cursor: Option<&str>) -> QueryResult<usize> {
    let Some(cursor) = cursor else {
        return Ok(0);
    };
    let cursor = cursor.trim();
    if cursor.is_empty() {
        return Err(QueryError::invalid_params("cursor must not be blank"));
    }

    cursor
        .parse::<usize>()
        .map_err(|_error| QueryError::invalid_params("cursor must be a non-negative offset"))
}

fn required_text(value: String, field_name: &str) -> QueryResult<String> {
    let trimmed = value.trim();

    if trimmed.is_empty() {
        return Err(QueryError::invalid_params(format!(
            "{field_name} must not be blank"
        )));
    }

    Ok(trimmed.to_string())
}

fn optional_text(value: Option<String>, field_name: &str) -> QueryResult<Option<String>> {
    match value {
        Some(value) => required_text(value, field_name).map(Some),
        None => Ok(None),
    }
}
