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

# Run the Axum server in dev mode
run-server-dev:
    cargo run -p quilt-server

# Build complete server binary with embedded frontend
build-server:
    cargo build -p quilt-server

# Build just the Rust backend (no bundling) - DEPRECATED
build-backend:
    cargo build -p quilt-platform

# Build frontend only (WASM via trunk)
build-frontend:
    cd crates/quilt-ui && trunk build

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

# Run all tests for workspace including doc tests
test-all:
    cargo test --workspace --all-features

# ── Lint ─────────────────────────────────────────────────────────────────

# Format code
fmt:
    cargo fmt --all

# Check formatting (CI mode, fails if not formatted)
fmt-check:
    cargo fmt --all -- --check

# Run clippy with strict warnings (library code only, no tests)
clippy:
    cargo clippy --lib --bins --all-features -- -D warnings

# Run clippy including tests
clippy-all:
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

# Run full CI pipeline: fmt + clippy (lib) + build + test
check: fmt-check clippy build test

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

# ── Clean ───────────────────────────────────────────────────────────────

# Clean all build artifacts
clean:
    cargo clean

# Clean and rebuild from scratch
rebuild: clean build

# Clean server build artifacts
clean-server:
    cargo clean -p quilt-server

# ── Docs ─────────────────────────────────────────────────────────────────

# Generate and open crate documentation
docs:
    cargo doc --no-deps --open

# ── Setup ─────────────────────────────────────────────────────────────────

# Install required tools (cargo-watch, cargo-llvm-cov)
setup:
    cargo install cargo-watch
    cargo install cargo-llvm-cov
    rustup component add rustfmt clippy llvm-tools

# Install Playwright for E2E testing
setup-e2e:
    cd e2e && npm install

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

# ── Scheduler ───────────────────────────────────────────────────────────

# Start the task scheduler (background tasks)
scheduler-start:
    cargo run -p quilt-bin -- scheduler start

# List all scheduled tasks
scheduler-list:
    cargo run -p quilt-bin -- scheduler list

# Run a scheduled task immediately
scheduler-now name:
    cargo run -p quilt-bin -- scheduler run-now {{name}}

# ── WASM / Frontend ──────────────────────────────────────────────────────────

# Build the Leptos UI for WASM target
build-wasm:
    cd crates/quilt-ui && trunk build

# Watch and serve WASM UI in dev mode
dev-wasm:
    cd crates/quilt-ui && trunk serve --port 1420

# ── E2E Tests (Tauri WebDriver) ─────────────────────────────────────────────

# Install E2E test dependencies (WebdriverIO + tauri-driver)
e2e-install:
    cargo install tauri-driver --locked
    cd e2e && npm install

# Run E2E tests using Tauri WebDriver (requires built app)
e2e-test:
    cd e2e && npm test

# Run E2E tests in headed mode (see browser)
e2e-test-headed:
    cd e2e && npm run test:headed || npx wdio run e2e/wdio.conf.ts --headed

# List all E2E tests without running
e2e-list:
    cd e2e && npx wdio run e2e/wdio.conf.ts --spec ./tests/**/*.ts --dry

# Debug E2E tests
e2e-debug:
    cd e2e && npx wdio debug e2e/wdio.conf.ts

# Run E2E tests with full app rebuild
e2e-all: build-server
    cd e2e && npm test

# Show E2E test report
e2e-report:
    @echo "E2E tests use WebdriverIO spec reporter - check console output"

# ── Desktop App ──────────────────────────────────────────────────────────────

# ── Axum Server (quilt-server) ────────────────────────────────────────────────

# Build quilt-server in release mode
build-server-release:
    cargo build -p quilt-server --release

# Run the quilt-server (Axum HTTP server)
serve:
    cargo run -p quilt-server

# Run quilt-server with custom port
serve-port port:
    QUILT_PORT={{port}} cargo run -p quilt-server

# Run quilt-server with CORS enabled (for development)
serve-cors:
    QUILT_CORS=true cargo run -p quilt-server

# Run quilt-server in release mode
run-server-release:
    cargo run -p quilt-server --release

# ── E2E Tests (Playwright) ──────────────────────────────────────────────────────

# Install Playwright for E2E testing
e2e-install-playwright:
    cd e2e && npm install

# Run E2E tests with Playwright (after quilt-server is running)
e2e-playwright:
    cd e2e && npx playwright test
