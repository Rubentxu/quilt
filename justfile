# Quilt — AI-first Knowledge Graph
# Build & development automation via just
#
# Usage:
#   just          → build debug
#   just release  → build release
#   just test     → run all tests
#   just check    → fmt + clippy + build + test (CI gate)
#   just dev      → build + watch + run server

default := "build"

# ── Build ────────────────────────────────────────────────────────────────

# Build debug binary
build:
    cargo build

# Build release binary
release:
    cargo build --release

# Build with all features
build-all:
    cargo build --all-features

# ── Dev / Watch ──────────────────────────────────────────────────────────

# Watch changes and rebuild automatically
watch:
    cargo watch -x build

# Watch and run tests on change
watch-test:
    cargo watch -x test

# Watch and run clippy on change
watch-lint:
    cargo watch -x clippy

# ── Run ──────────────────────────────────────────────────────────────────

# Run the CLI with default database
run +args="":
    cargo run -p quilt-bin -- {{args}}

# Run the MCP server on stdio (for AI agent integration)
run-server db_path="quilt.db":
    cargo run -p quilt-bin -- --db-path {{db_path}} serve

# Run the Tauri desktop app in dev mode
run-desktop:
    cd crates/quilt-platform/src-tauri && cargo tauri dev

# ── Test ─────────────────────────────────────────────────────────────────

# Run all unit tests
test:
    cargo test

# Run tests with output
test-verbose:
    cargo test -- --nocapture

# Run only integration tests
test-integration:
    cargo test --test '*'

# Run only doc tests
test-doc:
    cargo test --doc

# Run tests for a specific crate (e.g., just test-crate quilt-query)
test-crate crate:
    cargo test -p {{crate}}

# ── Lint ─────────────────────────────────────────────────────────────────

# Format code
fmt:
    cargo fmt --all

# Check formatting (CI mode, fails if not formatted)
fmt-check:
    cargo fmt --all -- --check

# Run clippy with strict warnings
clippy:
    cargo clippy --all-targets --all-features -- -D warnings

# Auto-fix clippy suggestions
clippy-fix:
    cargo clippy --all-targets --all-features --fix --allow-dirty --allow-staged

# ── Coverage ─────────────────────────────────────────────────────────────

# Generate code coverage report (requires cargo-llvm-cov)
coverage:
    cargo llvm-cov --all-features --lcov --output-path lcov.info

# Open coverage report in browser
coverage-html:
    cargo llvm-cov --all-features --html
    @echo "Open target/llvm-cov/html/index.html"

# ── Quality Gate (CI) ────────────────────────────────────────────────────

# Run full CI pipeline: fmt + clippy + build + test
check: fmt-check clippy test

# Full CI with all features
check-all: fmt-check
    cargo clippy --all-targets --all-features -- -D warnings
    cargo test --all-features

# ── Database ─────────────────────────────────────────────────────────────

# Create/reset the development database and run migrations
db-setup:
    cargo run -p quilt-bin -- db init

# Run pending migrations only
db-migrate:
    cargo run -p quilt-bin -- db migrate

# ── Clean ────────────────────────────────────────────────────────────────

# Clean all build artifacts
clean:
    cargo clean

# Clean and rebuild from scratch
rebuild: clean build

# ── Docs ──────────────────────────────────────────────────────────────────

# Generate and open crate documentation
docs:
    cargo doc --no-deps --open

# ── Setup ─────────────────────────────────────────────────────────────────

# Install required tools (cargo-watch, cargo-llvm-cov)
setup:
    cargo install cargo-watch
    cargo install cargo-llvm-cov
    rustup component add rustfmt clippy llvm-tools

# ── TreeRAG ──────────────────────────────────────────────────────────────

# Build TreeRAG block summary index for all pages
tree-rag-index:
    cargo run -p quilt-bin -- tree-rag index

# Build index for a specific page
tree-rag-index-page page:
    cargo run -p quilt-bin -- tree-rag index --page {{page}}

# Generate a structured report from the knowledge graph
report topic scope="auto":
    cargo run -p quilt-bin -- tree-rag report --topic "{{topic}}" --scope {{scope}}

# Generate a report and save as Markdown
report-md topic scope="auto" output="report.md":
    cargo run -p quilt-bin -- tree-rag report --topic "{{topic}}" --scope {{scope}} --output {{output}}

# Show TreeRAG index status
tree-rag-status:
    cargo run -p quilt-bin -- tree-rag status

# ── Scheduler ────────────────────────────────────────────────────────────

# Start the task scheduler (background tasks)
scheduler-start:
    cargo run -p quilt-bin -- scheduler start

# List all scheduled tasks
scheduler-list:
    cargo run -p quilt-bin -- scheduler list

# Run a scheduled task immediately
scheduler-now name:
    cargo run -p quilt-bin -- scheduler run-now {{name}}

# ── WASM ──────────────────────────────────────────────────────────────────

# Build the Yew UI for WASM target
build-wasm:
    cd crates/quilt-ui && trunk build

# Watch and serve WASM UI in dev mode
dev-wasm:
    cd crates/quilt-ui && trunk serve

# ── E2E Tests (Playwright) ─────────────────────────────────────────────────

# Install Playwright browsers
e2e-install:
    npm install && npx playwright install --with-deps

# Run E2E tests (requires dev server on port 1420)
e2e-test:
    npx playwright test

# Run E2E tests in headed mode (see browser)
e2e-test-headed:
    npx playwright test --headed

# List all E2E tests without running
e2e-list:
    npx playwright test --list

# Open Playwright UI
e2e-ui:
    npx playwright test --ui

# Run specific E2E test file
e2e-test-file file:
    npx playwright test {{file}}

# Run E2E tests with debug mode
e2e-debug:
    npx playwright test --debug

# Show E2E test report
e2e-report:
    npx playwright show-report

# Start dev server and run E2E tests
e2e-all:
    cd crates/quilt-ui && trunk serve --port 1420 &
    sleep 5
    npx playwright test
    pkill -f "trunk serve" || true
