CREATE TABLE fts_document_contents (
  document_id INTEGER PRIMARY KEY REFERENCES fts_documents(id),
  file_id INTEGER NOT NULL REFERENCES files(id),
  path TEXT NOT NULL,
  language TEXT NOT NULL CHECK (language IN ('rust', 'csharp', 'markdown', 'other')),
  content TEXT NOT NULL,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_fts_document_contents_file
  ON fts_document_contents(file_id);
