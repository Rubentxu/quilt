# ── Quilt development toolkit ──────────────────────────────────────────
# Install: cargo install just
# Usage:   just <command>

# ── Rust ──────────────────────────────────────────────────────────────

# Format all Rust code
fmt:
    cargo fmt

# Run clippy lints (warnings only, not blocking)
clippy:
    cargo clippy

# Run clippy with -D warnings (strict mode — may fail with preexisting debt)
clippy-strict:
    cargo clippy -- -D warnings

# Run all workspace tests
test:
    cargo test

# Check compilation (fast feedback)
check:
    cargo check

# Run fmt, clippy, test in sequence
ci:
    cargo fmt --check
    cargo clippy
    cargo test

# ── Server (quilt-server) ─────────────────────────────────────────────

# Build the backend server
server-build:
    cargo build -p quilt-server

# Run the backend server (default port 3737, CORS enabled for dev)
# Override: just server-dev QUILT_PORT=8080
server-dev:
    QUILT_GRAPH_DIR=. QUILT_CORS=true target/debug/quilt-server

# ── UI (Leptos + Trunk + Tailwind) ───────────────────────────────────

# Install UI toolchain (trunk + npm deps + CM6 bundle)
ui-deps:
    @which trunk >/dev/null 2>&1 || cargo binstall trunk -y
    cd crates/quilt-ui && npm install
    cd crates/quilt-ui/cm6 && npm install && node bundle.mjs

# Build Tailwind CSS (uses PostCSS — needed before trunk serve)
ui-css:
    cd crates/quilt-ui && npx postcss style.css -o dist/style.css

# Build CM6 editor bundle
ui-cm6:
    cd crates/quilt-ui/cm6 && npm install && node bundle.mjs

# Build UI for production (CSS + CM6 + trunk build)
ui-build: ui-css ui-cm6
    cd crates/quilt-ui && trunk build

# Run UI dev server (trunk serve with hot reload on port 8090)
# Requires: just ui-deps first, and server running in another terminal
ui-dev: ui-css ui-cm6
    cd crates/quilt-ui && trunk serve --port 8090 --open

# ── Full dev workflow ────────────────────────────────────────────────

# One-time setup: install all dependencies
setup: ui-deps server-build
    @echo ""
    @echo "✅ Setup complete! To start developing:"
    @echo ""
    @echo "  Terminal 1:  just server-dev"
    @echo "  Terminal 2:  just ui-dev"
    @echo ""
    @echo "  Then open http://localhost:8090"
    @echo "  Backend API at http://localhost:3737"

# ── E2E (Playwright smoke tests) ────────────────────────────────────

# Install E2E dependencies (Playwright + chromium)
e2e-deps:
    cd e2e && npm install
    npx playwright install chromium --with-deps

# Run E2E smoke tests (requires trunk dev server running on :8090)
# Start with: `just ui-dev` in another terminal
e2e:
    cd e2e && npx playwright test

# ── Housekeeping ─────────────────────────────────────────────────────

# Clean build artifacts
clean:
    cargo clean
    rm -rf crates/quilt-ui/dist

# Kill dev servers
stop-dev:
    @-kill $(lsof -ti:3737) 2>/dev/null; echo "Server stopped"
    @-kill $(lsof -ti:8090) 2>/dev/null; echo "Trunk stopped"

# Show available commands
default:
    @just --list
