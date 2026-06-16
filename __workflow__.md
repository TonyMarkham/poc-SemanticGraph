## WIP
```text
I want you to Audit the graphify repo in this repo's submodules with the following in mind:
- I am specifically interested in its feature that builds a semantic node graph.
- I want you to suggest an sqlite database schema to store a durable node graph.
- I understand that it uses a bespoke formula for generating its semantic model
- My coding projects are limited to Rust and C#, so I can use specific tools instead of building my own bespoke interogation system
- It is my intention to use `rust-analyzer` (source in this repo's submodules) as an LSP for building the semantic model for Rust workspaces
- It is my intention to use `csharp-language-server` (source in this repo's submodules) as an LSP for building the semantic model for C# solutions
```

```text
Check out the `semantic-graph-extract` CLI
Is `--workspace-root <WORKSPACE_ROOT>  [default: .]` actually necessary
```

---

```text
Create VS-1400.md as the implementation plan for Phase 4 from the VS-100.md being sure to use the directions stated in VS1000.md where appropriate

- Include a `Measurement Of Done` section with `- [ ]` style checkboxes as a final evaluation checklist.

## Measure Of Done Section Addendum
- Rust Code: Zero usage of `std::result::Result` for any return value. Custom {custom}Result<T> is acceptable in error modules.
- Rust Code: MUST only contain 1 type per file.
- `cargo fmt` has been run
- `cargo check` is clean: no errors or warnings
- `cargo clippy` is clean: no errors or warnings
- `cargo build` is clean: no errors or warnings
- `cargo build --release` is clean: no errors or warnings
- `cargo test` has no failing tests
```

---

```text
/goal VS-1400.md
```

---
