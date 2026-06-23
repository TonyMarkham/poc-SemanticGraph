-- no-transaction

PRAGMA foreign_keys = OFF;

DROP INDEX IF EXISTS idx_files_workspace_path;
DROP INDEX IF EXISTS idx_nodes_workspace_qname;
DROP INDEX IF EXISTS idx_nodes_file;
DROP INDEX IF EXISTS idx_fts_documents_workspace_active;
DROP INDEX IF EXISTS idx_fts_documents_file;
DROP INDEX IF EXISTS idx_fts_document_contents_file;

CREATE TABLE workspaces_new (
  id INTEGER PRIMARY KEY,
  root_uri TEXT NOT NULL UNIQUE,
  kind TEXT NOT NULL CHECK (kind IN ('rust', 'csharp', 'soul', 'mixed')),
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
INSERT INTO workspaces_new (id, root_uri, kind, created_at)
SELECT id, root_uri, kind, created_at
FROM workspaces;
DROP TABLE workspaces;
ALTER TABLE workspaces_new RENAME TO workspaces;

CREATE TABLE files_new (
  id INTEGER PRIMARY KEY,
  workspace_id INTEGER NOT NULL REFERENCES workspaces(id),
  uri TEXT NOT NULL,
  path TEXT NOT NULL,
  language TEXT NOT NULL CHECK (language IN ('rust', 'csharp', 'soul', 'markdown', 'other')),
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
  language,
  content_hash,
  last_seen_run_id,
  properties_json
FROM files;
DROP TABLE files;
ALTER TABLE files_new RENAME TO files;

CREATE TABLE nodes_new (
  id TEXT PRIMARY KEY,
  workspace_id INTEGER NOT NULL REFERENCES workspaces(id),
  language TEXT NOT NULL CHECK (language IN ('rust', 'csharp', 'soul', 'markdown', 'other')),
  kind TEXT NOT NULL,
  name TEXT NOT NULL,
  qualified_name TEXT,
  display_name TEXT,
  symbol_key TEXT NOT NULL,
  file_id INTEGER REFERENCES files(id),
  start_line INTEGER,
  start_col INTEGER,
  end_line INTEGER,
  end_col INTEGER,
  selection_start_line INTEGER,
  selection_start_col INTEGER,
  container_node_id TEXT REFERENCES nodes(id),
  properties_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(properties_json)),
  first_seen_run_id INTEGER REFERENCES extraction_runs(id),
  last_seen_run_id INTEGER REFERENCES extraction_runs(id),
  valid_to_run_id INTEGER REFERENCES extraction_runs(id),
  UNIQUE (workspace_id, language, symbol_key)
);
INSERT INTO nodes_new (
  id,
  workspace_id,
  language,
  kind,
  name,
  qualified_name,
  display_name,
  symbol_key,
  file_id,
  start_line,
  start_col,
  end_line,
  end_col,
  selection_start_line,
  selection_start_col,
  container_node_id,
  properties_json,
  first_seen_run_id,
  last_seen_run_id,
  valid_to_run_id
)
SELECT
  id,
  workspace_id,
  language,
  kind,
  name,
  qualified_name,
  display_name,
  symbol_key,
  file_id,
  start_line,
  start_col,
  end_line,
  end_col,
  selection_start_line,
  selection_start_col,
  container_node_id,
  properties_json,
  first_seen_run_id,
  last_seen_run_id,
  valid_to_run_id
FROM nodes;
DROP TABLE nodes;
ALTER TABLE nodes_new RENAME TO nodes;

CREATE TABLE fts_documents_new (
  id INTEGER PRIMARY KEY,
  workspace_id INTEGER NOT NULL REFERENCES workspaces(id),
  file_id INTEGER NOT NULL REFERENCES files(id),
  language TEXT NOT NULL CHECK (language IN ('rust', 'csharp', 'soul', 'markdown', 'other')),
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
  language,
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
  language TEXT NOT NULL CHECK (language IN ('rust', 'csharp', 'soul', 'markdown', 'other')),
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
  language,
  content,
  updated_at
FROM fts_document_contents;
DROP TABLE fts_document_contents;
ALTER TABLE fts_document_contents_new RENAME TO fts_document_contents;

CREATE INDEX idx_files_workspace_path
  ON files(workspace_id, path);

CREATE INDEX idx_nodes_workspace_qname
  ON nodes(workspace_id, qualified_name);

CREATE INDEX idx_nodes_file
  ON nodes(file_id);

CREATE INDEX idx_fts_documents_workspace_active
  ON fts_documents(workspace_id, valid_to_run_id);

CREATE INDEX idx_fts_documents_file
  ON fts_documents(file_id);

CREATE INDEX idx_fts_document_contents_file
  ON fts_document_contents(file_id);

PRAGMA foreign_key_check;
PRAGMA foreign_keys = ON;
