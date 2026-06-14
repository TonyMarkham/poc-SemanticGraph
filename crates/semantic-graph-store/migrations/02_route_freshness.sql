CREATE TABLE extraction_route_status (
  id INTEGER PRIMARY KEY,
  workspace_id INTEGER NOT NULL REFERENCES workspaces(id),
  route TEXT NOT NULL,
  scope TEXT NOT NULL CHECK (scope IN ('file', 'workspace')),
  scope_key TEXT NOT NULL,
  file_id INTEGER REFERENCES files(id),
  provider TEXT NOT NULL,
  provider_version TEXT,
  content_hash TEXT,
  last_started_run_id INTEGER REFERENCES extraction_runs(id),
  last_complete_run_id INTEGER REFERENCES extraction_runs(id),
  last_status TEXT NOT NULL CHECK (last_status IN ('running', 'complete', 'failed')),
  diagnostics_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(diagnostics_json)),
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE (workspace_id, route, scope, scope_key, provider)
);

CREATE TABLE route_observations (
  id INTEGER PRIMARY KEY,
  workspace_id INTEGER NOT NULL REFERENCES workspaces(id),
  run_id INTEGER NOT NULL REFERENCES extraction_runs(id),
  route TEXT NOT NULL,
  scope TEXT NOT NULL CHECK (scope IN ('file', 'workspace')),
  scope_key TEXT NOT NULL,
  provider TEXT NOT NULL,
  entity_kind TEXT NOT NULL CHECK (entity_kind IN ('node', 'edge')),
  entity_id TEXT NOT NULL,
  source_file_id INTEGER REFERENCES files(id),
  properties_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(properties_json)),
  UNIQUE (
    run_id,
    route,
    scope,
    scope_key,
    provider,
    entity_kind,
    entity_id,
    source_file_id
  )
);

CREATE INDEX idx_extraction_route_status_workspace_route
  ON extraction_route_status(workspace_id, route, scope, scope_key);

CREATE INDEX idx_route_observations_route_run
  ON route_observations(workspace_id, route, run_id, entity_kind);

CREATE INDEX idx_route_observations_entity
  ON route_observations(entity_kind, entity_id);
