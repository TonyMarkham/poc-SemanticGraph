set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

package := "semantic-graph-store"
extract_package := "semantic-graph-extract"
local_dir := ".local"
demo_db := ".local/semantic-graph-store-demo.db"
sqlx_db := ".local/sqlx-prepare.db"
sqlx_migrations := "crates/semantic-graph-store/migrations"

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
    just --justfile {{justfile()}} fmt
    SQLX_OFFLINE=true cargo check
    SQLX_OFFLINE=true cargo clippy --all-targets -- -D warnings
    SQLX_OFFLINE=true cargo build
    SQLX_OFFLINE=true cargo build --release
    SQLX_OFFLINE=true cargo test
    just --justfile {{justfile()}} confidence-rust-workspace

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
rust-workspace-references-smoke:
    mkdir -p {{local_dir}}
    rm -f .local/rust-workspace-references.db
    cargo run -p {{extract_package}} -- rust-workspace \
      --db .local/rust-workspace-references.db \
      --workspace-root . \
      --symbols
    cargo run -p {{extract_package}} -- rust-workspace \
      --db .local/rust-workspace-references.db \
      --workspace-root . \
      --references
    cargo run -p {{package}} -- stats --db .local/rust-workspace-references.db

# Exercise workspace-scoped rust-analyzer call extraction.
rust-workspace-calls-smoke:
    mkdir -p {{local_dir}}
    rm -f .local/rust-workspace-calls.db
    cargo run -p {{extract_package}} -- rust-workspace \
      --db .local/rust-workspace-calls.db \
      --workspace-root . \
      --symbols
    cargo run -p {{extract_package}} -- rust-workspace \
      --db .local/rust-workspace-calls.db \
      --workspace-root . \
      --calls
    cargo run -p {{package}} -- stats --db .local/rust-workspace-calls.db

# Exercise complete workspace extraction as part of confidence checks.
confidence-rust-workspace:
    ./target/release/semantic-graph-extract rust-workspace

# Exercise complete workspace-scoped rust-analyzer extraction in one CLI call.
rust-workspace-smoke:
    mkdir -p {{local_dir}}
    rm -f .local/rust-workspace-extract.db
    cargo run -p {{extract_package}} -- rust-workspace \
      --db .local/rust-workspace-extract.db \
      --workspace-root .
    cargo run -p {{package}} -- stats --db .local/rust-workspace-extract.db
