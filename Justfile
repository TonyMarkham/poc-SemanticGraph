set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

package := "semantic-graph-store"
extract_package := "semantic-graph-extract"
local_dir := ".local"
demo_db := ".local/semantic-graph-store-demo.db"
sqlx_db := ".local/sqlx-prepare.db"
sqlx_migrations := "crates/semantic-graph-store/migrations"
refactor_bin_dir := ".refactor-radar/bin"

# Show available recipes.
default:
    just --list

# Format the storage slice.
fmt:
    cargo fmt -p {{package}}

# Type-check the storage slice.
check:
    cargo check -p {{package}}

# Run clippy with workspace lint expectations.
clippy:
    cargo clippy -p {{package}} --all-targets -- -D warnings

# Run storage slice tests.
test:
    cargo test -p {{package}}

# Regenerate SQLx offline metadata after query or migration changes.
sqlx-prepare:
    mkdir -p {{local_dir}}
    rm -f {{sqlx_db}}
    DATABASE_URL=sqlite://{{sqlx_db}} cargo sqlx database create
    DATABASE_URL=sqlite://{{sqlx_db}} cargo sqlx migrate run --source {{sqlx_migrations}}
    DATABASE_URL=sqlite://{{sqlx_db}} cargo sqlx prepare --workspace -- --all-targets

# Fail if checked SQLx metadata is stale.
sqlx-check:
    mkdir -p {{local_dir}}
    rm -f {{sqlx_db}}
    DATABASE_URL=sqlite://{{sqlx_db}} cargo sqlx database create
    DATABASE_URL=sqlite://{{sqlx_db}} cargo sqlx migrate run --source {{sqlx_migrations}}
    DATABASE_URL=sqlite://{{sqlx_db}} cargo sqlx prepare --check --workspace -- --all-targets

# Exercise the CLI against a disposable SQLite database.
db-smoke db=demo_db:
    mkdir -p {{local_dir}}
    rm -f {{db}}
    cargo run -p {{package}} -- init --db {{db}}
    cargo run -p {{package}} -- demo-seed --db {{db}} --root-uri file:///tmp/poc-semanticgraph
    cargo run -p {{package}} -- stats --db {{db}}

# Main local confidence checker.
confidence:
    just --justfile {{justfile()}} sqlx-prepare
    cargo fmt
    cargo check
    cargo clippy --all-targets -- -D warnings
    cargo build
    cargo build --release
    cargo test
    just --justfile {{justfile()}} copy-release-bins
    just --justfile {{justfile()}} confidence-rust-workspace
    # just --justfile {{justfile()}} confidence-csharp-solution

# Copy release binaries into the project-local Refactor Radar bin directory.
copy-release-bins:
    mkdir -p {{refactor_bin_dir}}
    cp -f target/release/semantic-graph {{refactor_bin_dir}}/
    cp -f target/release/semantic-graph-agent-assets {{refactor_bin_dir}}/
    cp -f target/release/semantic-graph-extract {{refactor_bin_dir}}/
    cp -f target/release/semantic-graph-mcp-server {{refactor_bin_dir}}/
    cp -f target/release/semantic-graph-store {{refactor_bin_dir}}/

# Exercise rust-analyzer document-symbol extraction against crates/wip.
rust-extract-smoke:
    mkdir -p {{local_dir}}
    rm -f .local/rust-extract-wip.db
    cargo run -p {{extract_package}} -- rust-file \
      --db .local/rust-extract-wip.db \
      --workspace-root . \
      --symbols \
      crates/wip/src/lib.rs
    cargo run -p {{package}} -- stats --db .local/rust-extract-wip.db

# Exercise crate-scoped rust-analyzer document-symbol extraction against crates/wip.
rust-crate-extract-smoke:
    mkdir -p {{local_dir}}
    rm -f .local/rust-crate-extract-wip.db
    cargo run -p {{extract_package}} -- rust-crate \
      --db .local/rust-crate-extract-wip.db \
      --workspace-root . \
      --symbols \
      crates/wip
    cargo run -p {{package}} -- stats --db .local/rust-crate-extract-wip.db

# Exercise workspace-scoped rust-analyzer document-symbol extraction.
rust-workspace-extract-smoke:
    mkdir -p {{local_dir}}
    rm -f .local/rust-workspace-extract.db
    cargo run -p {{extract_package}} -- rust-workspace \
      --db .local/rust-workspace-extract.db \
      --workspace-root . \
      --symbols
    cargo run -p {{package}} -- stats --db .local/rust-workspace-extract.db

# Exercise workspace-scoped rust-analyzer reference extraction.
rust-workspace-reference-route-smoke:
    mkdir -p {{local_dir}}
    rm -f .local/rust-workspace-reference-route.db
    cargo run -p {{extract_package}} -- rust-workspace \
      --db .local/rust-workspace-reference-route.db \
      --workspace-root . \
      --symbols
    cargo run -p {{extract_package}} -- rust-workspace \
      --db .local/rust-workspace-reference-route.db \
      --workspace-root . \
      --references
    cargo run -p {{package}} -- stats --db .local/rust-workspace-reference-route.db

# Exercise workspace-scoped rust-analyzer call extraction.
rust-workspace-call-route-smoke:
    mkdir -p {{local_dir}}
    rm -f .local/rust-workspace-call-route.db
    cargo run -p {{extract_package}} -- rust-workspace \
      --db .local/rust-workspace-call-route.db \
      --workspace-root . \
      --symbols
    cargo run -p {{extract_package}} -- rust-workspace \
      --db .local/rust-workspace-call-route.db \
      --workspace-root . \
      --calls
    cargo run -p {{package}} -- stats --db .local/rust-workspace-call-route.db

# Exercise complete csharp-ls solution extraction against the local C# fixture.
csharp-solution-smoke:
    mkdir -p {{local_dir}}
    rm -f .local/csharp-solution-extract.db
    cargo run -p {{extract_package}} -- csharp-solution \
      --db .local/csharp-solution-extract.db \
      --solution __SmokeTestAssets__/csharp-wip/CSharpWip.sln
    cargo run -p {{package}} -- stats --db .local/csharp-solution-extract.db

# Exercise solution-scoped csharp-ls reference extraction.
csharp-solution-reference-route-smoke:
    mkdir -p {{local_dir}}
    rm -f .local/csharp-solution-reference-route.db
    cargo run -p {{extract_package}} -- csharp-solution \
      --db .local/csharp-solution-reference-route.db \
      --solution __SmokeTestAssets__/csharp-wip/CSharpWip.sln \
      --symbols
    cargo run -p {{extract_package}} -- csharp-solution \
      --db .local/csharp-solution-reference-route.db \
      --solution __SmokeTestAssets__/csharp-wip/CSharpWip.sln \
      --references
    cargo run -p {{package}} -- stats --db .local/csharp-solution-reference-route.db

# Exercise solution-scoped csharp-ls incoming-call extraction.
csharp-solution-call-route-smoke:
    mkdir -p {{local_dir}}
    rm -f .local/csharp-solution-call-route.db
    cargo run -p {{extract_package}} -- csharp-solution \
      --db .local/csharp-solution-call-route.db \
      --solution __SmokeTestAssets__/csharp-wip/CSharpWip.sln \
      --symbols
    cargo run -p {{extract_package}} -- csharp-solution \
      --db .local/csharp-solution-call-route.db \
      --solution __SmokeTestAssets__/csharp-wip/CSharpWip.sln \
      --calls
    cargo run -p {{package}} -- stats --db .local/csharp-solution-call-route.db

# Exercise complete csharp-ls single-file extraction against the local C# fixture.
csharp-file-smoke:
    mkdir -p {{local_dir}}
    rm -f .local/csharp-file-extract.db
    cargo run -p {{extract_package}} -- csharp-file \
      --db .local/csharp-file-extract.db \
      --solution __SmokeTestAssets__/csharp-wip/CSharpWip.sln \
      __SmokeTestAssets__/csharp-wip/Project/Worker.cs
    cargo run -p {{package}} -- stats --db .local/csharp-file-extract.db

# Exercise C# deleted-file stale marking against the local C# fixture.
csharp-file-deleted-smoke:
    mkdir -p {{local_dir}}
    rm -f .local/csharp-file-deleted.db
    cargo run -p {{extract_package}} -- csharp-file \
      --db .local/csharp-file-deleted.db \
      --solution __SmokeTestAssets__/csharp-wip/CSharpWip.sln \
      __SmokeTestAssets__/csharp-wip/Project/Worker.cs
    cargo run -p {{extract_package}} -- csharp-file-deleted \
      --db .local/csharp-file-deleted.db \
      --solution __SmokeTestAssets__/csharp-wip/CSharpWip.sln \
      __SmokeTestAssets__/csharp-wip/Project/Worker.cs
    cargo run -p {{package}} -- stats --db .local/csharp-file-deleted.db

# Exercise complete workspace extraction as part of confidence checks.
confidence-rust-workspace:
    ./target/release/semantic-graph-extract rust-workspace

# Exercise complete C# solution extraction as part of confidence checks.
confidence-csharp-solution:
    ./target/release/semantic-graph-extract csharp-solution

# Exercise complete workspace-scoped rust-analyzer extraction in one CLI call.
rust-workspace-smoke:
    mkdir -p {{local_dir}}
    rm -f .local/rust-workspace-extract.db
    cargo run -p {{extract_package}} -- rust-workspace \
      --db .local/rust-workspace-extract.db \
      --workspace-root .
    cargo run -p {{package}} -- stats --db .local/rust-workspace-extract.db
