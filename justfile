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
REACT_PORT := "5173"

# React frontend paths
REACT_DIR := ROOT / "quilt-ui"
WASM_PKG_DIR := REACT_DIR / "src" / "core" / "wasm-bridge" / "pkg"

# Working data directory (overridable via QUILT_GRAPH_DIR)
QUILT_DIR := "$HOME/.quilt-dev"

# Binary paths
SERVER_BINARY := ROOT / "target" / "debug" / "quilt-server"

# Frontend env file
ENV_FILE := REACT_DIR / ".env"

# Private: ensure API key exists, print it (calls scripts/ensure-api-key.sh)
_api-key:
	@bash scripts/ensure-api-key.sh

# -----------------------------------------------------------------------------
# Default recipe - show help
# -----------------------------------------------------------------------------

default:
    @echo "Quilt - Available commands:"
    @echo ""
    @echo "=== QUICK START ==="
    @echo "  just setup               Install ALL deps (one-time)"
    @echo "  just dev                 Rebuild ALL + start servers (React + WASM watch)"
    @echo "  just dev-fast            Start servers without rebuilding (faster)"
    @echo "  just dev-react           Rebuild ALL + start servers (React + WASM watch)"
    @echo ""
    @echo "=== BACKEND (quilt-server) ==="
    @echo "  just server-build        Build the server"
    @echo "  just server-dev          Run server (foreground)"
    @echo "  just server-start        Run server (background)"
    @echo "  just server-stop         Stop background server"
    @echo "  just server-logs         Show server logs"
    @echo "  just server-status       Check if server is running"
    @echo ""
    @echo "=== FRONTEND (React + Vite) ==="
    @echo "  just react-deps          Install React dependencies"
    @echo "  just react-wasm          Build WASM package once"
    @echo "  just react-wasm-watch    Watch Rust changes, rebuild WASM"
    @echo "  just react-dev           Vite only (assumes backend running)"
    @echo "  just dev-react           Build + start all (backend + WASM watch + Vite)"
    @echo "  just react-build         Production build"
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
    @echo "  just coverage            Generate HTML + LCOV coverage report"
    @echo "  just coverage-ci         Generate LCOV + summary (CI mode)"
    @echo "  just coverage-clean      Remove coverage artefacts"
    @echo ""
    @echo "=== HOT RELOAD NOTES ==="
    @echo "  Rust changes:   Auto-detected by Trunk (rebuilds WASM + CSS + CM6)"
    @echo "  CSS changes:    just dev starts a CSS watcher (or run: just ui-css)"
    @echo "  CM6 JS changes: Run 'just ui-cm6' manually (NOT watched by Trunk)"
    @echo "  Full rebuild:   just dev  (rebuilds everything from scratch)"
    @echo ""
    @echo "=== FULL LIST ==="
    @echo "  just --list             Show all recipes"

# =============================================================================
# Quick Start — Full Dev Environment
# =============================================================================

# Install ALL dependencies (one-time setup)
setup: server-build react-deps
    @echo ""
    @echo "✅ Setup complete!"
    @echo ""
    @echo "  → Start developing:  just dev"
    @echo ""
    @echo "  Terminal 1 (backend):  http://localhost:{{ SERVER_PORT }}"
    @echo "    Health check:         http://localhost:{{ SERVER_PORT }}/health"
    @echo "    API:                  http://localhost:{{ SERVER_PORT }}/api/v1"
    @echo ""
    @echo "  Terminal 2 (frontend): http://localhost:{{ REACT_PORT }}"

# Start backend without rebuilding frontend assets (faster)
dev-fast: stop-all
    @echo ""
    @echo "═══ Starting servers (fast mode) ═══"
    @echo ""
    @echo "▶ Backend..."
    cargo build -p quilt-server
    @mkdir -p {{ QUILT_DIR }}
    @QUILT_API_KEY=$$(just _api-key 2>/dev/null || true) QUILT_GRAPH_DIR={{ QUILT_DIR }} QUILT_CORS=true \
        RUST_LOG=quilt_server=info \
        {{ SERVER_BINARY }} > {{ QUILT_DIR }}/server.log 2>&1 &
    @sleep 2
    @curl -sf http://localhost:{{ SERVER_PORT }}/health > /dev/null 2>&1 && echo "  ✓ Backend — http://localhost:{{ SERVER_PORT }}" || (echo "  ✗ Backend failed" && exit 1)
    @echo ""
    @echo "  ▶ Backend running. Start frontend with: just react-dev"
    @echo "  ▶ Or use: just dev-react (full rebuild + start)"

# Start both backend and frontend for development
# Kills any existing processes, rebuilds everything, then starts fresh
dev: dev-react

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
    QUILT_API_KEY=$$(just _api-key 2>/dev/null || true) QUILT_GRAPH_DIR={{ QUILT_DIR }} QUILT_CORS=true RUST_LOG=quilt_server=info,tower_http=info {{ SERVER_BINARY }}

# Run server in background (for use with ui-dev)
# Kills any existing process on the port first
server-start:
    @-kill $(lsof -ti:{{ SERVER_PORT }}) 2>/dev/null
    @echo "Starting server on :{{ SERVER_PORT }}..."
    @mkdir -p {{ QUILT_DIR }}
    @QUILT_API_KEY=$$(just _api-key 2>/dev/null || true) QUILT_GRAPH_DIR={{ QUILT_DIR }} QUILT_CORS=true \
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
    @-kill $(lsof -ti:{{ SERVER_PORT }}) 2>/dev/null && echo "  Server stopped" || echo "  Server not running"

# Check if server is running
server-status:
    @if curl -sf http://localhost:{{ SERVER_PORT }}/health > /dev/null 2>&1; then \
        echo "✓ Server running — http://localhost:{{ SERVER_PORT }}"; \
        echo "  Health: $(curl -s http://localhost:{{ SERVER_PORT }}/health)"; \
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
# React Frontend (quilt-ui)
# ---------------------------------------------------------------------------

# Install React dependencies
react-deps:
    cd {{ REACT_DIR }} && npm install

# Build quilt-core WASM package for React
react-wasm:
    wasm-pack build {{ ROOT }}/crates/quilt-core --target web --out-dir {{ WASM_PKG_DIR }} --dev

# Build quilt-core WASM package for production
react-wasm-release:
    wasm-pack build {{ ROOT }}/crates/quilt-core --target web --out-dir {{ WASM_PKG_DIR }} --release

# Watch Rust changes and rebuild WASM automatically (runs in foreground)
react-wasm-watch:
    @echo "👀 Watching quilt-core/ for WASM rebuilds..."
    cargo watch \
        --watch crates/quilt-core/src \
        --watch crates/quilt-core/Cargo.toml \
        --shell "wasm-pack build crates/quilt-core --target web --out-dir '{{ WASM_PKG_DIR }}' --dev && echo '✅ WASM rebuilt' || echo '❌ Build failed'"

# Run React dev server only (assumes backend + WASM already built)
react-dev:
    cd {{ REACT_DIR }} && npx vite --port {{ REACT_PORT }}

# Full React development environment:
# 1. Builds backend server
# 2. Builds WASM package
# 3. Starts backend (background)
# 4. Starts WASM watcher (background)
# 5. Starts Vite dev server (foreground)
# Ctrl+C stops everything cleanly.
dev-react: stop-all
    {{ ROOT }}/scripts/dev-react.sh

# Build React for production
react-build: react-wasm-release
    cd {{ REACT_DIR }} && npx tsc -b && npx vite build

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
    QUILT_API_KEY=$$(just _api-key 2>/dev/null || true) npx playwright test

# Run E2E with visible browser
test-e2e-headed:
    QUILT_API_KEY=$$(just _api-key 2>/dev/null || true) npx playwright test --headed --project=chromium

# Run E2E in Playwright UI mode
test-e2e-ui:
    npx playwright test --ui

# Run E2E with debug mode
test-e2e-debug:
    npx playwright test --debug

# Show E2E test report
test-e2e-report:
    npx playwright show-report

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
# Coverage (cargo-llvm-cov)
# ---------------------------------------------------------------------------

# Generate the full coverage report (HTML + LCOV + text + summary)
coverage:
    bash scripts/coverage.sh

# Generate only the artefacts CI needs (LCOV + summary)
coverage-ci:
    cargo llvm-cov --workspace --lcov --output-path coverage/lcov.info --ignore-run-fail
    cargo llvm-cov --workspace --summary-only --ignore-run-fail

# Clean coverage artefacts
coverage-clean:
    rm -rf coverage coverage.txt
    @echo "✓ Coverage artefacts removed"

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
    @echo "Opening Quilt at http://localhost:{{ REACT_PORT }}"
    @which xdg-open > /dev/null && xdg-open http://localhost:{{ REACT_PORT }} || \
    which open > /dev/null && open http://localhost:{{ REACT_PORT }} || \
    echo "Open http://localhost:{{ REACT_PORT }} manually"

# Test API directly via curl
api-test:
    @echo "── Health ──"
    @curl -s http://localhost:{{ SERVER_PORT }}/health | python3 -m json.tool 2>/dev/null || curl -s http://localhost:{{ SERVER_PORT }}/health
    @echo ""
    @echo "── Pages ──"
    @API_KEY=$$(just _api-key 2>/dev/null || echo ""); curl -s -H "Authorization: Bearer $$API_KEY" http://localhost:{{ SERVER_PORT }}/api/v1/pages | python3 -m json.tool 2>/dev/null || curl -s -H "Authorization: Bearer $$API_KEY" http://localhost:{{ SERVER_PORT }}/api/v1/pages

# Clean all build artifacts
clean:
    cargo clean
    @echo "✓ Cleaned"

# Stop ALL dev processes
stop-all:
    @echo "Stopping all dev servers..."
    @-kill $(lsof -ti:{{ SERVER_PORT }}) 2>/dev/null && echo "  ✓ Backend stopped" || echo "  Backend not running"
    @-pkill -f "quilt-server" 2>/dev/null && echo "  ✓ Server process stopped" || echo "  Server process not running"
    @-kill $(lsof -ti:{{ REACT_PORT }}) 2>/dev/null && echo "  ✓ Vite stopped" || echo "  Vite not running"
    @-pkill -f "cargo watch.*quilt-core" 2>/dev/null && echo "  ✓ WASM watcher stopped" || echo "  WASM watcher not running"
    @-pkill -f "cargo-watch" 2>/dev/null
    @echo "✓ All dev servers stopped"

# Show watchers for development
dev-watch:
    @echo "Starting watch mode..."
    @echo "  Backend: cargo watch -x 'build -p quilt-server'"
    @echo "  Frontend: use: just react-wasm-watch + just react-dev"

# Show development status
dev-status:
    @echo "=== Quilt Development Status ==="
    @echo ""
    @echo "Backend (port {{ SERVER_PORT }}):"
    @-curl -sf http://localhost:{{ SERVER_PORT }}/health > /dev/null && \
        echo "  ✓ Running — $(curl -s http://localhost:{{ SERVER_PORT }}/health)" || \
        echo "  ✗ Not running"
    @echo ""
    @echo "Frontend (port {{ REACT_PORT }}):"
    @-curl -sf http://localhost:{{ REACT_PORT }} > /dev/null && \
        echo "  ✓ Running" || \
        echo "  ✗ Not running"
    @echo ""
    @echo "Processes:"
    @-ps aux | grep -E "(quilt-server|vite)" | grep -v grep || echo "  No processes"
    @echo ""
    @echo "Ports:"
    @-ss -tlnp | grep -E "{{ SERVER_PORT }}|{{ REACT_PORT }}" || echo "  No ports in use"

# Show all recipes (alias)
help:
    @just --list

# =============================================================================
# Containerized Testing (Podman)
# =============================================================================

# Build the test container image (all test layers: Rust + Vitest + Playwright)
test-container-build:
    podman build -t quilt:test -f Containerfile.test .
    @echo "✓ Test container built: quilt:test"

# Run Rust unit + integration tests inside container
test-container-rust:
    podman run --rm -v .:/src:Z quilt:test cargo test

# Run Rust tests for a specific crate inside container
test-container-crate crate:
    podman run --rm -v .:/src:Z quilt:test cargo test -p {{ crate }}

# Run frontend component tests (vitest) inside container
test-container-vitest:
    podman run --rm -v .:/src:Z -w /src/quilt-ui quilt:test npx vitest run

# Run E2E Playwright tests inside container
test-container-e2e:
    podman run --rm -v .:/src:Z --network host quilt:test npx playwright test

# Run E2E smoke tests inside container
test-container-e2e-smoke:
    podman run --rm -v .:/src:Z --network host quilt:test npx playwright test --grep @smoke

# Run ALL tests inside container (Rust + Vitest + E2E)
test-container-all:
    podman run --rm -v .:/src:Z --network host quilt:test bash -c \
        "cargo test && cd quilt-ui && npx vitest run && cd .. && npx playwright test --grep @smoke"
    @echo "✓ All containerized tests passed"

# Open an interactive shell in the test container
test-container-shell:
    podman run --rm -it -v .:/src:Z quilt:test bash

# Run the full test suite without building the container first
test-container-quick:
    podman run --rm -v .:/src:Z --network host quilt:test cargo test

# =============================================================================
# Quadlet Management
# =============================================================================

QUADLET_DIR := "{{ env_var('HOME') }}/.config/containers/systemd"

# Install quadlet for local test execution (auto-starts on login if enabled)
quadlet-install-test:
    @mkdir -p {{ QUADLET_DIR }}
    cp quadlets/quilt-test.container {{ QUADLET_DIR }}/
    systemctl --user daemon-reload
    @echo "✓ Quadlet installed"
    @echo "  Enable auto-start:  systemctl --user enable --now quilt-test"
    @echo "  Run once:           systemctl --user start quilt-test"
    @echo "  Check status:       systemctl --user status quilt-test"
    @echo "  View logs:          journalctl --user -u quilt-test -f"

# Install production quadlet
quadlet-install-prod:
    @mkdir -p {{ QUADLET_DIR }}
    @echo "[Container]" > {{ QUADLET_DIR }}/quilt.container
    @echo "Image=quilt:latest" >> {{ QUADLET_DIR }}/quilt.container
    @echo "PublishPort=3737:3737" >> {{ QUADLET_DIR }}/quilt.container
    @echo "Volume=quilt-data:/home/appuser/.quilt-data" >> {{ QUADLET_DIR }}/quilt.container
    @echo "Environment=RUST_LOG=info" >> {{ QUADLET_DIR }}/quilt.container
    @echo "HealthInterval=30s" >> {{ QUADLET_DIR }}/quilt.container
    @echo "AutoUpdate=registry" >> {{ QUADLET_DIR }}/quilt.container
    @echo "" >> {{ QUADLET_DIR }}/quilt.container
    @echo "[Service]" >> {{ QUADLET_DIR }}/quilt.container
    @echo "Restart=always" >> {{ QUADLET_DIR }}/quilt.container
    systemctl --user daemon-reload
    @echo "✓ Production quadlet installed"

# Remove all quadlets
quadlet-remove-all:
    -rm -f {{ QUADLET_DIR }}/quilt.container {{ QUADLET_DIR }}/quilt-test.container
    -systemctl --user stop quilt quilt-test 2>/dev/null || true
    systemctl --user daemon-reload
    @echo "✓ Quadlets removed"

# Show quadlet status
quadlet-status:
    @echo "=== Quadlet Containers ==="
    @-systemctl --user status quilt-test --no-pager 2>/dev/null || echo "  quilt-test: not running"
    @echo ""
    @-systemctl --user status quilt --no-pager 2>/dev/null || echo "  quilt: not running"
    @echo ""
    @echo "=== Installed Quadlets ==="
    @-ls {{ QUADLET_DIR }}/quilt*.container 2>/dev/null || echo "  None installed"

# Run tests via quadlet (one-shot, container auto-removes)
quadlet-test:
    systemctl --user start quilt-test
    @echo "✓ Test started. Check: systemctl --user status quilt-test"
    @echo "  Logs: journalctl --user -u quilt-test -f"

# Build + install + run tests via quadlet
quadlet-test-full: container-build test-container-build quadlet-install quadlet-test
    @echo "✓ Full quadlet test pipeline started"

# =============================================================================
# Integration Testing (requires running server)
# =============================================================================
# These recipes use podman to spin up a Quilt server container,
# run integration tests (WebSocket, navigate, API), then tear down.
#
# Quick start:
#   just test-integration    Build image + run all integration tests

INTEG_PORT := "3737"

# Build the server image for integration testing
test-integration-build:
    podman build -t quilt:server -f Containerfile --target runtime .
    @echo "✓ Server image built: quilt:server"

# Start test server in background container
test-integration-start:
    @-podman rm -f quilt-test-server 2>/dev/null || true
    podman run -d --name quilt-test-server \
        -p {{ INTEG_PORT }}:{{ INTEG_PORT }} \
        -v quilt-test-data:/home/appuser/.quilt-data \
        -e RUST_LOG=info \
        quilt:server
    @echo "Waiting for server..."
    @for i in 1 2 3 4 5 6 7 8 9 10; do \
        if curl -sf http://localhost:{{ INTEG_PORT }}/health > /dev/null 2>&1; then \
            echo "✓ Server ready — http://localhost:{{ INTEG_PORT }}"; \
            break; \
        fi; \
        sleep 1; \
    done

# Stop and remove test server container
test-integration-stop:
    -podman stop quilt-test-server 2>/dev/null
    -podman rm quilt-test-server 2>/dev/null
    @echo "✓ Test server stopped"

# Show test server logs
test-integration-logs:
    podman logs --tail 50 quilt-test-server

# Run WebSocket integration tests (requires server running)
test-ws:
    cargo test -p quilt-server --test ws_integration_tests -- --nocapture

# Run navigate integration tests (requires server running)
test-navigate:
    cargo test -p quilt-server --test navigate_integration_tests -- --nocapture

# Full integration test pipeline: build → start → test → stop
test-integration: test-integration-build test-integration-start
    @echo ""
    @echo "═══ Running integration tests ═══"
    -cargo test -p quilt-server --test ws_integration_tests -- --nocapture
    -cargo test -p quilt-server --test navigate_integration_tests -- --nocapture
    @echo ""
    @just test-integration-stop
    @echo "✓ Integration tests complete"

# Run integration tests without rebuilding the image
test-integration-quick: test-integration-start
    -cargo test -p quilt-server --test ws_integration_tests -- --nocapture
    -cargo test -p quilt-server --test navigate_integration_tests -- --nocapture
    @just test-integration-stop

# Install test server quadlet (for integration tests)
quadlet-install-test-server:
    @mkdir -p {{ QUADLET_DIR }}
    cp quadlets/quilt-test-server.container {{ QUADLET_DIR }}/
    systemctl --user daemon-reload
    @echo "✓ Test server quadlet installed"
    @echo "  Start:  systemctl --user start quilt-test-server"
    @echo "  Stop:   systemctl --user stop quilt-test-server"
    @echo "  Status: systemctl --user status quilt-test-server"
