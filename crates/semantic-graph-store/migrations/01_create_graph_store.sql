PRAGMA foreign_keys = ON;

CREATE TABLE workspaces (
  id INTEGER PRIMARY KEY,
  root_uri TEXT NOT NULL UNIQUE,
  kind TEXT NOT NULL CHECK (kind IN ('rust', 'csharp', 'mixed')),
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE extraction_runs (
  id INTEGER PRIMARY KEY,
  workspace_id INTEGER NOT NULL REFERENCES workspaces(id),
  provider TEXT NOT NULL,
  provider_version TEXT,
  git_commit TEXT,
  started_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  finished_at TEXT,
  status TEXT NOT NULL DEFAULT 'running'
    CHECK (status IN ('running', 'complete', 'failed')),
  properties_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(properties_json))
);

CREATE TABLE files (
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

CREATE TABLE nodes (
  id TEXT PRIMARY KEY,
  workspace_id INTEGER NOT NULL REFERENCES workspaces(id),
  language TEXT NOT NULL CHECK (language IN ('rust', 'csharp', 'markdown', 'other')),
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

CREATE TABLE edges (
  id TEXT PRIMARY KEY,
  workspace_id INTEGER NOT NULL REFERENCES workspaces(id),
  src_node_id TEXT NOT NULL REFERENCES nodes(id),
  dst_node_id TEXT NOT NULL REFERENCES nodes(id),
  relation TEXT NOT NULL,
  context TEXT,
  confidence TEXT NOT NULL
    CHECK (confidence IN ('EXTRACTED', 'INFERRED', 'AMBIGUOUS')),
  confidence_score REAL NOT NULL,
  weight REAL NOT NULL DEFAULT 1.0,
  properties_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(properties_json)),
  first_seen_run_id INTEGER REFERENCES extraction_runs(id),
  last_seen_run_id INTEGER REFERENCES extraction_runs(id),
  valid_to_run_id INTEGER REFERENCES extraction_runs(id),
  UNIQUE (workspace_id, src_node_id, dst_node_id, relation, context)
);

CREATE TABLE edge_evidence (
  id INTEGER PRIMARY KEY,
  edge_id TEXT NOT NULL REFERENCES edges(id),
  run_id INTEGER NOT NULL REFERENCES extraction_runs(id),
  provider TEXT NOT NULL,
  lsp_method TEXT,
  file_id INTEGER REFERENCES files(id),
  start_line INTEGER,
  start_col INTEGER,
  end_line INTEGER,
  end_col INTEGER,
  raw_json TEXT CHECK (raw_json IS NULL OR json_valid(raw_json))
);

CREATE TABLE occurrences (
  id INTEGER PRIMARY KEY,
  node_id TEXT NOT NULL REFERENCES nodes(id),
  run_id INTEGER NOT NULL REFERENCES extraction_runs(id),
  file_id INTEGER NOT NULL REFERENCES files(id),
  role TEXT NOT NULL CHECK (
    role IN (
      'definition',
      'declaration',
      'reference',
      'call',
      'import',
      'implementation',
      'override'
    )
  ),
  start_line INTEGER NOT NULL,
  start_col INTEGER NOT NULL,
  end_line INTEGER NOT NULL,
  end_col INTEGER NOT NULL,
  enclosing_node_id TEXT REFERENCES nodes(id),
  raw_json TEXT CHECK (raw_json IS NULL OR json_valid(raw_json))
);

CREATE INDEX idx_files_workspace_path
  ON files(workspace_id, path);

CREATE INDEX idx_nodes_workspace_qname
  ON nodes(workspace_id, qualified_name);

CREATE INDEX idx_nodes_file
  ON nodes(file_id);

CREATE INDEX idx_edges_src
  ON edges(src_node_id);

CREATE INDEX idx_edges_dst
  ON edges(dst_node_id);

CREATE INDEX idx_edges_relation
  ON edges(workspace_id, relation);

CREATE INDEX idx_occurrences_node_role
  ON occurrences(node_id, role);

CREATE INDEX idx_occurrences_file
  ON occurrences(file_id);

CREATE INDEX idx_edge_evidence_edge
  ON edge_evidence(edge_id);

CREATE VIRTUAL TABLE node_search USING fts5(
  node_id UNINDEXED,
  name,
  qualified_name,
  display_name,
  file_path
);
