-- no-transaction

PRAGMA foreign_keys = OFF;

DROP INDEX IF EXISTS idx_files_workspace_path;
DROP INDEX IF EXISTS idx_fts_documents_workspace_active;
DROP INDEX IF EXISTS idx_fts_documents_file;
DROP INDEX IF EXISTS idx_fts_document_contents_file;

CREATE TABLE files_new (
  id INTEGER PRIMARY KEY,
  workspace_id INTEGER NOT NULL REFERENCES workspaces(id),
  uri TEXT NOT NULL,
  path TEXT NOT NULL,
  language TEXT NOT NULL CHECK (language IN ('rust', 'csharp', 'markdown', 'other')),
  content_hash TEXT,
  last_seen_run_id INTEGER REFERENCES extraction_runs(id),
  properties_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(properties_json)),
  UNIQUE (workspace_id, uri)
);
INSERT INTO files_new (
  id,
  workspace_id,
  uri,
  path,
  language,
  content_hash,
  last_seen_run_id,
  properties_json
)
SELECT
  id,
  workspace_id,
  uri,
  path,
  CASE
    WHEN lower(path) GLOB '*.rs' THEN 'rust'
    WHEN lower(path) GLOB '*.cs' THEN 'csharp'
    WHEN lower(path) GLOB '*.csx' THEN 'csharp'
    WHEN lower(path) GLOB '*.md' THEN 'markdown'
    WHEN lower(path) GLOB '*.markdown' THEN 'markdown'
    WHEN lower(path) GLOB '*.mdx' THEN 'markdown'
    WHEN language = 'soul' THEN 'other'
    ELSE language
  END,
  content_hash,
  last_seen_run_id,
  properties_json
FROM files;
DROP TABLE files;
ALTER TABLE files_new RENAME TO files;

CREATE TABLE fts_documents_new (
  id INTEGER PRIMARY KEY,
  workspace_id INTEGER NOT NULL REFERENCES workspaces(id),
  file_id INTEGER NOT NULL REFERENCES files(id),
  language TEXT NOT NULL CHECK (language IN ('rust', 'csharp', 'markdown', 'other')),
  content_hash TEXT NOT NULL,
  byte_len INTEGER NOT NULL,
  indexed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  first_seen_run_id INTEGER REFERENCES extraction_runs(id),
  last_seen_run_id INTEGER REFERENCES extraction_runs(id),
  valid_to_run_id INTEGER REFERENCES extraction_runs(id),
  properties_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(properties_json)),
  UNIQUE (workspace_id, file_id)
);
INSERT INTO fts_documents_new (
  id,
  workspace_id,
  file_id,
  language,
  content_hash,
  byte_len,
  indexed_at,
  first_seen_run_id,
  last_seen_run_id,
  valid_to_run_id,
  properties_json
)
SELECT
  id,
  workspace_id,
  file_id,
  CASE
    WHEN lower((SELECT path FROM files WHERE files.id = fts_documents.file_id)) GLOB '*.rs'
      THEN 'rust'
    WHEN lower((SELECT path FROM files WHERE files.id = fts_documents.file_id)) GLOB '*.cs'
      THEN 'csharp'
    WHEN lower((SELECT path FROM files WHERE files.id = fts_documents.file_id)) GLOB '*.csx'
      THEN 'csharp'
    WHEN lower((SELECT path FROM files WHERE files.id = fts_documents.file_id)) GLOB '*.md'
      THEN 'markdown'
    WHEN lower((SELECT path FROM files WHERE files.id = fts_documents.file_id)) GLOB '*.markdown'
      THEN 'markdown'
    WHEN lower((SELECT path FROM files WHERE files.id = fts_documents.file_id)) GLOB '*.mdx'
      THEN 'markdown'
    WHEN language = 'soul' THEN 'other'
    ELSE language
  END,
  content_hash,
  byte_len,
  indexed_at,
  first_seen_run_id,
  last_seen_run_id,
  valid_to_run_id,
  properties_json
FROM fts_documents;
DROP TABLE fts_documents;
ALTER TABLE fts_documents_new RENAME TO fts_documents;

CREATE TABLE fts_document_contents_new (
  document_id INTEGER PRIMARY KEY REFERENCES fts_documents(id),
  file_id INTEGER NOT NULL REFERENCES files(id),
  path TEXT NOT NULL,
  language TEXT NOT NULL CHECK (language IN ('rust', 'csharp', 'markdown', 'other')),
  content TEXT NOT NULL,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
INSERT INTO fts_document_contents_new (
  document_id,
  file_id,
  path,
  language,
  content,
  updated_at
)
SELECT
  document_id,
  file_id,
  path,
  CASE
    WHEN lower(path) GLOB '*.rs' THEN 'rust'
    WHEN lower(path) GLOB '*.cs' THEN 'csharp'
    WHEN lower(path) GLOB '*.csx' THEN 'csharp'
    WHEN lower(path) GLOB '*.md' THEN 'markdown'
    WHEN lower(path) GLOB '*.markdown' THEN 'markdown'
    WHEN lower(path) GLOB '*.mdx' THEN 'markdown'
    WHEN language = 'soul' THEN 'other'
    ELSE language
  END,
  content,
  updated_at
FROM fts_document_contents;
DROP TABLE fts_document_contents;
ALTER TABLE fts_document_contents_new RENAME TO fts_document_contents;

CREATE INDEX idx_files_workspace_path
  ON files(workspace_id, path);

CREATE INDEX idx_fts_documents_workspace_active
  ON fts_documents(workspace_id, valid_to_run_id);

CREATE INDEX idx_fts_documents_file
  ON fts_documents(file_id);

CREATE INDEX idx_fts_document_contents_file
  ON fts_document_contents(file_id);

PRAGMA foreign_key_check;
PRAGMA foreign_keys = ON;
