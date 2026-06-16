## Commands

C# refreshes use `semantic-graph-extract` rather than MCP tools.

- `csharp-file`: refresh one C# file, with one route selector at a time.
- `csharp-file-deleted`: record zero observations for a removed C# file and soft-close file-scoped facts.
- `csharp-project`: refresh selected routes for one project boundary.
- `csharp-solution`: refresh selected routes for the solution.

`csharp-project` and `csharp-solution` support combinable `--symbols`, `--references`, and `--calls` selectors. Relation-only route runs require the selected files' symbol graph to already exist unless `--symbols` is selected in the same invocation.

## Routes

- `csharp.document_symbols`
- `csharp.references`
- `csharp.calls`

C# extraction uses `csharp-language-server`. Incoming call hierarchy is implemented by that server, but outgoing call hierarchy has returned no result in the local source evidence; do not assume C# outgoing call edges are available through that path without verifying the current server source and route output.

Use `--solution` when discovery is ambiguous and `--csharp-ls` when the language-server binary is not on `PATH`. Report the command, route selectors, summary counts, and validation performed after any approved refresh.
