# =============================================================================
# Quilt development toolkit
# =============================================================================
# Comprehensive automation for building, testing, and running Quilt.
# Modeled after agents-workflows dev infrastructure.
#
# Usage: just <recipe>
# Show all recipes: just --list

# -----------------------------------------------------------------------------
# Configuration
# -----------------------------------------------------------------------------

ROOT := justfile_directory()

# Ports
SERVER_PORT := "3737"
UI_PORT := "8090"
UI_E2E_PORT := "8090"

# Working data directory (overridable via QUILT_GRAPH_DIR)
QUILT_DIR := "$HOME/.quilt-dev"

# Binary paths
SERVER_BINARY := ROOT / "target" / "debug" / "quilt-server"

# -----------------------------------------------------------------------------
# Default recipe - show help
# -----------------------------------------------------------------------------

default:
    @echo "Quilt - Available commands:"
    @echo ""
    @echo "=== QUICK START ==="
    @echo "  just setup               Install ALL deps (one-time)"
    @echo "  just dev                 Start backend + frontend in parallel"
    @echo ""
    @echo "=== BACKEND (quilt-server) ==="
    @echo "  just server-build        Build the server"
    @echo "  just server-dev          Run server (foreground)"
    @echo "  just server-start        Run server (background)"
    @echo "  just server-stop         Stop background server"
    @echo "  just server-logs         Show server logs"
    @echo "  just server-status       Check if server is running"
    @echo ""
    @echo "=== FRONTEND (quilt-ui) ==="
    @echo "  just ui-deps             Install UI npm deps"
    @echo "  just ui-css              Build Tailwind CSS"
    @echo "  just ui-cm6              Build CodeMirror 6 bundle"
    @echo "  just ui-dev              Run UI dev server (hot reload)"
    @echo "  just ui-build            Build UI for production"
    @echo ""
    @echo "=== TESTING ==="
    @echo "  just test                Run all Rust unit tests"
    @echo "  just test-e2e            Run E2E smoke tests (Playwright)"
    @echo "  just test-e2e-headed     Run E2E with visible browser"
    @echo "  just test-e2e-ui         Run E2E in Playwright UI mode"
    @echo "  just test-all            Rust tests + E2E"
    @echo ""
    @echo "=== CONTAINER (Podman/Quadlet) ==="
    @echo "  just container-build     Build container image"
    @echo "  just container-run       Run container (background)"
    @echo "  just container-stop      Stop container"
    @echo "  just container-logs      Show container logs"
    @echo "  just quadlet-install     Install quadlet for auto-start"
    @echo ""
    @echo "=== CODE QUALITY ==="
    @echo "  just fmt                 Format all Rust code"
    @echo "  just check               Check compilation (fast)"
    @echo "  just clippy             Run clippy lints"
    @echo "  just ci                  Full CI pipeline (fmt + clippy + test)"
    @echo ""
    @echo "=== FULL LIST ==="
    @echo "  just --list             Show all recipes"

# =============================================================================
# Quick Start — Full Dev Environment
# =============================================================================

# Install ALL dependencies (one-time setup)
setup: server-build ui-deps
    @echo ""
    @echo "✅ Setup complete!"
    @echo ""
    @echo "  → Start developing:  just dev"
    @echo ""
    @echo "  Terminal 1 (backend):  http://localhost:{{ SERVER_PORT }}"
    @echo "    Health check:         http://localhost:{{ SERVER_PORT }}/health"
    @echo "    API:                  http://localhost:{{ SERVER_PORT }}/api/v1"
    @echo ""
    @echo "  Terminal 2 (frontend): http://localhost:{{ UI_PORT }}"
    @echo "    Journal (today):      http://localhost:{{ UI_PORT }}/journal"
    @echo "    Pages:                http://localhost:{{ UI_PORT }}/pages"

# Start both backend and frontend for development
# Runs server in background, UI in foreground (Ctrl+C stops UI)
dev: server-start
    @echo ""
    @echo "Backend running on :{{ SERVER_PORT }}"
    @echo "Starting frontend..."
    @echo ""
    cd {{ ROOT }}/crates/quilt-ui && trunk serve --port {{ UI_PORT }} --open

# ---------------------------------------------------------------------------
# Backend (quilt-server)
# ---------------------------------------------------------------------------

# Build the backend server (debug)
server-build:
    cargo build -p quilt-server

# Build the backend server (release)
server-build-release:
    cargo build --release -p quilt-server

# Run server in foreground (Ctrl+C to stop)
server-dev:
    QUILT_GRAPH_DIR={{ QUILT_DIR }} QUILT_CORS=true RUST_LOG=quilt_server=info,tower_http=info {{ SERVER_BINARY }}

# Run server in background (for use with ui-dev)
server-start:
    @echo "Starting server on :{{ SERVER_PORT }}..."
    @mkdir -p {{ QUILT_DIR }}
    @QUILT_GRAPH_DIR={{ QUILT_DIR }} QUILT_CORS=true \
        RUST_LOG=quilt_server=info \
        {{ SERVER_BINARY }} > {{ QUILT_DIR }}/server.log 2>&1 &
    @sleep 2
    @if curl -sf http://localhost:{{ SERVER_PORT }}/health > /dev/null 2>&1; then \
        echo "  ✓ Server running — http://localhost:{{ SERVER_PORT }}"; \
    else \
        echo "  ✗ Server failed to start. Check logs: tail -f {{ QUILT_DIR }}/server.log"; \
    fi

# Stop background server
server-stop:
    @-kill $$(lsof -ti:{{ SERVER_PORT }}) 2>/dev/null && echo "  Server stopped" || echo "  Server not running"

# Check if server is running
server-status:
    @if curl -sf http://localhost:{{ SERVER_PORT }}/health > /dev/null 2>&1; then \
        echo "✓ Server running — http://localhost:{{ SERVER_PORT }}"; \
        echo "  Health: $$(curl -s http://localhost:{{ SERVER_PORT }}/health)"; \
    else \
        echo "✗ Server not running"; \
    fi

# Show server logs
server-logs:
    tail -50 {{ QUILT_DIR }}/server.log 2>/dev/null || echo "No server logs found"

# Follow server logs
server-logs-follow:
    tail -f {{ QUILT_DIR }}/server.log 2>/dev/null || echo "No server logs found"

# ---------------------------------------------------------------------------
# Frontend (quilt-ui)
# ---------------------------------------------------------------------------

# Install UI dependencies (npm + CM6 bundle)
ui-deps:
    cd {{ ROOT }}/crates/quilt-ui && npm install
    cd {{ ROOT }}/crates/quilt-ui/cm6 && npm install

# Build Tailwind CSS (via PostCSS)
ui-css:
    cd {{ ROOT }}/crates/quilt-ui && npx postcss style.css -o dist/style.css

# Build CodeMirror 6 editor bundle
ui-cm6:
    cd {{ ROOT }}/crates/quilt-ui/cm6 && npm install && node bundle.mjs

# Watch Tailwind CSS (rebuild on style.css changes)
ui-css-watch:
    cd {{ ROOT }}/crates/quilt-ui && npx postcss style.css -o dist/style.css --watch

# Build all UI assets (CSS + CM6 bundle)
ui-assets: ui-css ui-cm6

# Run UI dev server (hot reload, requires backend running)
ui-dev: ui-assets
    cd {{ ROOT }}/crates/quilt-ui && trunk serve --port {{ UI_PORT }} --open

# Build UI for production deployment
ui-build: ui-assets
    cd {{ ROOT }}/crates/quilt-ui && trunk build

# Check UI compilation (WASM target, fast)
ui-check:
    cargo build -p quilt-ui --target wasm32-unknown-unknown

# ---------------------------------------------------------------------------
# Testing
# ---------------------------------------------------------------------------

# Run all Rust unit tests
test:
    cargo test

# Run Rust tests for a specific crate (e.g. just test-crate quilt-ui)
test-crate crate:
    cargo test -p {{ crate }}

# Run Rust tests with output (no capture)
test-nocapture:
    cargo test -- --nocapture

# Run E2E smoke tests via Playwright
test-e2e:
    cd {{ ROOT }}/e2e && npm install && npx playwright test

# Run E2E with visible browser
test-e2e-headed:
    cd {{ ROOT }}/e2e && npx playwright test --headed

# Run E2E in Playwright UI mode
test-e2e-ui:
    cd {{ ROOT }}/e2e && npx playwright test --ui

# Run E2E with debug mode
test-e2e-debug:
    cd {{ ROOT }}/e2e && npx playwright test --debug

# Show E2E test report
test-e2e-report:
    cd {{ ROOT }}/e2e && npx playwright show-report

# Full test suite: Rust tests + E2E
test-all: test test-e2e
    @echo "✓ All tests passed"

# ---------------------------------------------------------------------------
# Code Quality
# ---------------------------------------------------------------------------

# Format all Rust code
fmt:
    cargo fmt

# Check formatting (CI mode)
fmt-check:
    cargo fmt --check

# Run clippy lints
clippy:
    cargo clippy

# Run clippy with strict warnings
clippy-strict:
    cargo clippy -- -D warnings

# Check compilation (fast feedback)
check:
    cargo check

# Check compilation for WASM target (UI)
check-wasm:
    cargo check -p quilt-ui --target wasm32-unknown-unknown

# Full CI: format check + clippy + tests
ci:
    cargo fmt --check
    cargo clippy
    cargo test

# ---------------------------------------------------------------------------
# Container (Podman)
# ---------------------------------------------------------------------------

# Build container image for the server
container-build:
    podman build -t quilt:latest -f Containerfile .
    @echo "✓ Container built: quilt:latest"

# Run container in background
container-run:
    podman run -d \
        --name quilt \
        -p {{ SERVER_PORT }}:{{ SERVER_PORT }}/tcp \
        -v quilt-data:/home/appuser/.quilt-data \
        -e RUST_LOG=info \
        --restart=always \
        quilt:latest
    @echo "✓ Container running"
    @echo "  Health: http://localhost:{{ SERVER_PORT }}/health"

# Stop container
container-stop:
    -podman stop quilt 2>/dev/null
    @echo "✓ Container stopped"

# Remove container and volume
container-remove:
    -podman rm quilt 2>/dev/null
    -podman volume rm quilt-data 2>/dev/null
    @echo "✓ Container removed"

# Show container status
container-status:
    @podman ps --filter "name=quilt" 2>/dev/null || echo "No container running"
    @echo ""
    @podman volume ls --filter "name=quilt" 2>/dev/null || true

# Show container logs
container-logs:
    podman logs --tail 100 -f quilt

# ---------------------------------------------------------------------------
# Podman Quadlet (systemd integration)
# ---------------------------------------------------------------------------

QUADLET_USER_DIR := "$HOME/.config/containers/systemd"

# Install quadlet for auto-start at login
quadlet-install:
    @mkdir -p {{ QUADLET_USER_DIR }}
    @echo "[Container]" > {{ QUADLET_USER_DIR }}/quilt.container
    @echo "Image=quilt:latest" >> {{ QUADLET_USER_DIR }}/quilt.container
    @echo "PublishPort={{ SERVER_PORT }}:{{ SERVER_PORT }}" >> {{ QUADLET_USER_DIR }}/quilt.container
    @echo "Volume=quilt-data:/home/appuser/.quilt-data" >> {{ QUADLET_USER_DIR }}/quilt.container
    @echo "Environment=RUST_LOG=info" >> {{ QUADLET_USER_DIR }}/quilt.container
    @echo "AutoUpdate=registry" >> {{ QUADLET_USER_DIR }}/quilt.container
    @echo "" >> {{ QUADLET_USER_DIR }}/quilt.container
    @echo "[Service]" >> {{ QUADLET_USER_DIR }}/quilt.container
    @echo "Restart=always" >> {{ QUADLET_USER_DIR }}/quilt.container
    systemctl --user daemon-reload
    @echo "✓ Quadlet installed. Enable with: systemctl --user enable --now quilt"

# Remove quadlet
quadlet-remove:
    -rm -f {{ QUADLET_USER_DIR }}/quilt.container
    -systemctl --user stop quilt 2>/dev/null || true
    systemctl --user daemon-reload
    @echo "✓ Quadlet removed"

# ---------------------------------------------------------------------------
# Development Tools
# ---------------------------------------------------------------------------

# Open the Quilt Studio UI in browser
studio:
    @echo "Opening Quilt at http://localhost:{{ UI_PORT }}"
    @which xdg-open > /dev/null && xdg-open http://localhost:{{ UI_PORT }} || \
    which open > /dev/null && open http://localhost:{{ UI_PORT }} || \
    echo "Open http://localhost:{{ UI_PORT }} manually"

# Test API directly via curl
api-test:
    @echo "── Health ──"
    @curl -s http://localhost:{{ SERVER_PORT }}/health | python3 -m json.tool 2>/dev/null || curl -s http://localhost:{{ SERVER_PORT }}/health
    @echo ""
    @echo "── Pages ──"
    @curl -s http://localhost:{{ SERVER_PORT }}/api/v1/pages | python3 -m json.tool 2>/dev/null || curl -s http://localhost:{{ SERVER_PORT }}/api/v1/pages

# Clean all build artifacts
clean:
    cargo clean
    rm -rf {{ ROOT }}/crates/quilt-ui/dist
    rm -rf {{ ROOT }}/crates/quilt-ui/cm6/dist
    @echo "✓ Cleaned"

# Stop ALL dev processes (server + trunk)
stop-all: server-stop
    @-kill $$(lsof -ti:{{ UI_PORT }}) 2>/dev/null && echo "  UI server stopped" || echo "  UI server not running"
    @echo "✓ All dev servers stopped"

# Show watchers for development
dev-watch: ui-css ui-cm6
    @echo "Starting watch mode..."
    @echo "  Backend: cargo watch -x 'build -p quilt-server'"
    @echo "  Frontend: cd crates/quilt-ui && trunk watch --port {{ UI_PORT }}"
    cd {{ ROOT }}/crates/quilt-ui && trunk watch --port {{ UI_PORT }}

# Show development status
dev-status:
    @echo "=== Quilt Development Status ==="
    @echo ""
    @echo "Backend (port {{ SERVER_PORT }}):"
    @-curl -sf http://localhost:{{ SERVER_PORT }}/health > /dev/null && \
        echo "  ✓ Running — $$(curl -s http://localhost:{{ SERVER_PORT }}/health)" || \
        echo "  ✗ Not running"
    @echo ""
    @echo "Frontend (port {{ UI_PORT }}):"
    @-curl -sf http://localhost:{{ UI_PORT }} > /dev/null && \
        echo "  ✓ Running" || \
        echo "  ✗ Not running"
    @echo ""
    @echo "Processes:"
    @-ps aux | grep -E "(quilt-server|trunk)" | grep -v grep || echo "  No processes"
    @echo ""
    @echo "Ports:"
    @-ss -tlnp | grep -E "{{ SERVER_PORT }}|{{ UI_PORT }}" || echo "  No ports in use"

# Show all recipes (alias)
help:
    @just --list
