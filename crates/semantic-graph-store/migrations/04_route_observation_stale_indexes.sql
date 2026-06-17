CREATE INDEX idx_route_observations_route_scope_entity_run
  ON route_observations(
    workspace_id,
    route,
    scope,
    scope_key,
    provider,
    entity_kind,
    entity_id,
    run_id
  );

CREATE INDEX idx_route_observations_route_source_file_entity_run
  ON route_observations(
    workspace_id,
    route,
    provider,
    entity_kind,
    source_file_id,
    entity_id,
    run_id
  );
