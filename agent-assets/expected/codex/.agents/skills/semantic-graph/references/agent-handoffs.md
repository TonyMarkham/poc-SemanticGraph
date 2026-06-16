# Agent Handoffs

## Required Caller Context

When delegating to a SemanticGraph custom agent, provide:

- the database path or the configured MCP server context;
- the workspace root when refresh commands are allowed;
- whether mutation is allowed;
- the exact MCP tools or CLI commands the agent may use;
- the files, symbols, routes, or plan sections in scope.

## Agent Boundaries

Read-only agents must not run extraction commands or mutate SQLite. Refresh agents may run extraction commands only when the caller explicitly allows mutation and provides the command boundary.

Refresh agents must report the command, route selectors, summary counts, and validation performed. Plan and audit outputs should use `Confirmed`, `Inferred`, `Unknown`, and `Validation` sections when precision matters.

Agents must not rely on temporary host handoff files unless a generated host variant explicitly defines that mechanism.
