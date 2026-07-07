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
When running `semantic-graph-extract rust-workspace --symbols` can we calculate a file hash per file and save it to the db

Then, when we do subsequent passes, we can test the current file hash against the db hash. If same, exclude that file from any further extraction work
```

---

```text
Create VS-1500.md as the implementation plan for Phase 5 from the VS-100.md being sure to use the directions stated in VS1000.md where appropriate

- Include a `Measurement Of Done` section with `- [ ]` style checkboxes as a final evaluation checklist.

## Definition Of Done Section Addendum
- Rust Code: Zero usage of `std::result::Result` for any return value. Custom {custom}Result<T> is acceptable in error modules. ALso acceptable in tests.
- Rust Code: MUST only contain 1 type per file.
- `cargo fmt` has been run
- `cargo check` is clean: no errors or warnings
- `cargo clippy` is clean: no errors or warnings
- `cargo build` is clean: no errors or warnings
- `cargo build --release` is clean: no errors or warnings
- `cargo test` has no failing tests
```

---

## Init

```text
/goal Implement `VS-20.md` into a COMPLETE, professional and production-ready implementation.

**CRITICAL** Be sure to use all rust coding directives found in AGENTS.md

Feel free to create a temporary worktree if you want to experiment.
```

---
investigate
