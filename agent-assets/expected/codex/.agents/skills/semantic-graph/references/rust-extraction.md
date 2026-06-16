# Rust Extraction

## Commands

Rust refreshes use `semantic-graph-extract` rather than MCP tools.

- `rust-file`: refresh symbols, references, and calls for one Rust file by default.
- `rust-file-deleted`: record zero observations for a removed Rust file and soft-close file-scoped facts.
- `rust-crate`: refresh selected routes for one crate boundary.
- `rust-workspace`: refresh selected routes for the workspace.

`rust-crate` and `rust-workspace` support combinable `--symbols`, `--references`, and `--calls` selectors. With no selector, they refresh symbols, references, and calls. Relation-only `--references` and `--calls` runs require the selected files' symbol graph to already exist unless `--symbols` is selected in the same invocation.

## Routes

- `rust.document_symbols`
- `rust.references`
- `rust.calls`

Use `--workspace-root` when the workspace root is not the current directory. Use `--db` for explicit temporary or validation databases. Report the command, route selectors, summary counts, and validation performed after any approved refresh.
