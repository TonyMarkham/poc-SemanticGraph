use semantic_graph_search_tantivy::{
    TantivyFtsDocument, TantivyFtsIndex, TantivyFtsIndexUpdate, TantivyFtsSearchRequest,
};
use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[test]
fn search_returns_stored_metadata_with_filters_and_pagination() -> Result<(), Box<dyn Error>> {
    let root = temp_dir("tantivy-search")?;
    let index_path = root.join("fts.tantivy");
    let index = TantivyFtsIndex::open_or_create(&index_path)?;
    index.apply_update(TantivyFtsIndexUpdate {
        documents: vec![
            document(
                "file:///fixture/src/lib.rs",
                "src/lib.rs",
                "rust",
                "lib-hash",
                "alpha needle token\nExactCaseToken\n",
            ),
            document(
                "file:///fixture/docs/readme.md",
                "docs/readme.md",
                "markdown",
                "readme-hash",
                "needle token in docs\n",
            ),
            document(
                "file:///fixture/src/other.rs",
                "src/other.rs",
                "rust",
                "other-hash",
                "needle token in other\n",
            ),
            document(
                "file:///fixture/tie/a.txt",
                "tie/a.txt",
                "text",
                "a-hash",
                "stabletie marker\n",
            ),
            document(
                "file:///fixture/tie/b.txt",
                "tie/b.txt",
                "text",
                "b-hash",
                "stabletie marker\n",
            ),
        ],
        deleted_uris: Vec::new(),
        indexing_workers: 1,
    })?;

    let read_only_index = TantivyFtsIndex::open_read_only(&index_path)?;
    let first_page = read_only_index.search(TantivyFtsSearchRequest {
        query: "needle".to_string(),
        limit: 1,
        offset: 0,
        language: None,
        path_prefix: None,
        case_sensitive: false,
    })?;
    assert_eq!(1, first_page.hits.len());
    assert_eq!("file:///fixture/docs/readme.md", first_page.hits[0].uri);
    assert!(first_page.has_more);

    let second_page = read_only_index.search(TantivyFtsSearchRequest {
        query: "needle".to_string(),
        limit: 2,
        offset: 1,
        language: None,
        path_prefix: None,
        case_sensitive: false,
    })?;
    assert_eq!(2, second_page.hits.len());
    assert!(!second_page.has_more);

    let tie_first_page = read_only_index.search(TantivyFtsSearchRequest {
        query: "stabletie".to_string(),
        limit: 1,
        offset: 0,
        language: None,
        path_prefix: None,
        case_sensitive: false,
    })?;
    assert_eq!("file:///fixture/tie/a.txt", tie_first_page.hits[0].uri);
    assert!(tie_first_page.has_more);

    let tie_second_page = read_only_index.search(TantivyFtsSearchRequest {
        query: "stabletie".to_string(),
        limit: 1,
        offset: 1,
        language: None,
        path_prefix: None,
        case_sensitive: false,
    })?;
    assert_eq!("file:///fixture/tie/b.txt", tie_second_page.hits[0].uri);
    assert!(!tie_second_page.has_more);

    let rust_src_hits = read_only_index.search(TantivyFtsSearchRequest {
        query: "needle".to_string(),
        limit: 10,
        offset: 0,
        language: Some("rust".to_string()),
        path_prefix: Some("src/".to_string()),
        case_sensitive: false,
    })?;
    let mut rust_src_paths = rust_src_hits
        .hits
        .iter()
        .map(|hit| hit.path.clone())
        .collect::<Vec<_>>();
    rust_src_paths.sort();
    assert_eq!(
        vec!["src/lib.rs".to_string(), "src/other.rs".to_string()],
        rust_src_paths
    );

    let case_insensitive = read_only_index.search(TantivyFtsSearchRequest {
        query: "exactcasetoken".to_string(),
        limit: 10,
        offset: 0,
        language: None,
        path_prefix: None,
        case_sensitive: false,
    })?;
    assert_eq!(1, case_insensitive.hits.len());
    assert_eq!("file:///fixture/src/lib.rs", case_insensitive.hits[0].uri);

    let case_sensitive = read_only_index.search(TantivyFtsSearchRequest {
        query: "exactcasetoken".to_string(),
        limit: 10,
        offset: 0,
        language: None,
        path_prefix: None,
        case_sensitive: true,
    })?;
    assert!(case_sensitive.hits.is_empty());

    remove_dir(&root)?;
    Ok(())
}

#[test]
fn open_read_only_does_not_create_missing_index() -> Result<(), Box<dyn Error>> {
    let root = temp_dir("tantivy-read-only-missing")?;
    let index_path = root.join("missing.tantivy");

    let result = TantivyFtsIndex::open_read_only(&index_path);

    assert!(result.is_err());
    assert!(!index_path.exists());
    remove_dir(&root)?;
    Ok(())
}

fn document(
    uri: &str,
    path: &str,
    language: &str,
    content_hash: &str,
    content: &str,
) -> TantivyFtsDocument {
    TantivyFtsDocument {
        uri: uri.to_string(),
        path: path.to_string(),
        language: language.to_string(),
        content_hash: content_hash.to_string(),
        content: content.to_string(),
    }
}

fn temp_dir(name: &str) -> Result<PathBuf, Box<dyn Error>> {
    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let path = std::env::temp_dir().join(format!(
        "semantic-graph-search-tantivy-{name}-{nanos}-{counter}"
    ));
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn remove_dir(path: &Path) -> Result<(), Box<dyn Error>> {
    if path.exists() {
        fs::remove_dir_all(path)?;
    }
    Ok(())
}
