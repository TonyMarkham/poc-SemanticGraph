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
    SQLX_OFFLINE=true cargo check -p {{package}}
    SQLX_OFFLINE=true cargo clippy -p {{package}} --all-targets -- -D warnings
    SQLX_OFFLINE=true cargo test -p {{package}}
    SQLX_OFFLINE=true cargo check -p {{extract_package}}
    SQLX_OFFLINE=true cargo clippy -p {{extract_package}} --all-targets -- -D warnings
    SQLX_OFFLINE=true cargo test -p {{extract_package}}
    just --justfile {{justfile()}} db-smoke

# Exercise rust-analyzer document-symbol extraction against crates/wip.
rust-extract-smoke:
    mkdir -p {{local_dir}}
    rm -f .local/rust-extract-wip.db
    cargo run -p {{extract_package}} -- rust-document-symbols \
      --db .local/rust-extract-wip.db \
      --workspace-root . \
      --package-path crates/wip \
      --file crates/wip/src/lib.rs
    cargo run -p {{package}} -- stats --db .local/rust-extract-wip.db

# Exercise crate-scoped rust-analyzer document-symbol extraction against crates/wip.
rust-crate-extract-smoke:
    mkdir -p {{local_dir}}
    rm -f .local/rust-crate-extract-wip.db
    cargo run -p {{extract_package}} -- rust-crate-document-symbols \
      --db .local/rust-crate-extract-wip.db \
      --workspace-root . \
      --package-path crates/wip
    cargo run -p {{package}} -- stats --db .local/rust-crate-extract-wip.db

# Exercise workspace-scoped rust-analyzer document-symbol extraction.
rust-workspace-extract-smoke:
    mkdir -p {{local_dir}}
    rm -f .local/rust-workspace-extract.db
    cargo run -p {{extract_package}} -- rust-workspace-document-symbols \
      --db .local/rust-workspace-extract.db \
      --workspace-root .
    cargo run -p {{package}} -- stats --db .local/rust-workspace-extract.db

# Exercise workspace-scoped rust-analyzer reference extraction.
rust-workspace-references-smoke:
    mkdir -p {{local_dir}}
    rm -f .local/rust-workspace-references.db
    cargo run -p {{extract_package}} -- rust-workspace-document-symbols \
      --db .local/rust-workspace-references.db \
      --workspace-root .
    cargo run -p {{extract_package}} -- rust-workspace-references \
      --db .local/rust-workspace-references.db \
      --workspace-root .
    cargo run -p {{package}} -- stats --db .local/rust-workspace-references.db

# Exercise workspace-scoped rust-analyzer call extraction.
rust-workspace-calls-smoke:
    mkdir -p {{local_dir}}
    rm -f .local/rust-workspace-calls.db
    cargo run -p {{extract_package}} -- rust-workspace-document-symbols \
      --db .local/rust-workspace-calls.db \
      --workspace-root .
    cargo run -p {{extract_package}} -- rust-workspace-calls \
      --db .local/rust-workspace-calls.db \
      --workspace-root .
    cargo run -p {{package}} -- stats --db .local/rust-workspace-calls.db

# Exercise complete workspace-scoped rust-analyzer extraction in one CLI call.
rust-workspace-all-smoke:
    mkdir -p {{local_dir}}
    rm -f .local/rust-workspace-extract.db
    cargo run -p {{extract_package}} -- rust-workspace-all \
      --db .local/rust-workspace-extract.db \
      --workspace-root .
    cargo run -p {{package}} -- stats --db .local/rust-workspace-extract.db
