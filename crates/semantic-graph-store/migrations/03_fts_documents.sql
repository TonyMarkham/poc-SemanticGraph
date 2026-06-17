CREATE TABLE fts_documents (
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

CREATE VIRTUAL TABLE fts_document_trigram_ci USING fts5(
  document_id UNINDEXED,
  file_id UNINDEXED,
  path,
  language,
  content,
  tokenize = 'trigram case_sensitive 0'
);

CREATE INDEX idx_fts_documents_workspace_active
  ON fts_documents(workspace_id, valid_to_run_id);

CREATE INDEX idx_fts_documents_file
  ON fts_documents(file_id);
