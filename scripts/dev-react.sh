#!/usr/bin/env bash
# dev-react.sh — Quilt dev environment with full hot reload.
# Usage: just dev  OR  scripts/dev-react.sh
#
# Starts and supervises:
#   1. API key setup (auto-generate if missing, sync to .env)
#   2. quilt-server (backend, port 3737) — REST + WebSocket
#   3. Vite dev server (frontend, port 5173) — HMR for React
#   4. cargo-watch for quilt-core → rebuilds the WASM module on Rust changes
#   5. Node-based asset watcher → re-bundles and re-syncs into wasm_assets/
#
# Two URLs to use:
#   - http://localhost:5173  ← DEV with HMR (preferred, latest code)
#   - http://localhost:3737  ← Auto-rebuilt production bundle
#
# Both URLs are kept fresh at all times:
#   5173 = Vite dev (HMR for React)
#   3737 = Rust server reading from wasm_assets/, kept in sync by the
#          Node watcher
#
# Ctrl+C stops everything cleanly.

set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
QUILT_DIR="${QUILT_DIR:-$HOME/.quilt-dev}"
SERVER_PORT=3737
REACT_PORT=5173
WASM_PKG_DIR="$ROOT/quilt-ui/src/core/wasm-bridge/pkg"
ENV_FILE="$ROOT/quilt-ui/.env"

cleanup() {
    echo ""
    echo "Stopping all services..."
    kill $(lsof -ti:$SERVER_PORT) 2>/dev/null && echo "  ✓ Backend stopped" || true
    kill $(lsof -ti:$REACT_PORT) 2>/dev/null && echo "  ✓ Vite stopped" || true
    pkill -P $$ 2>/dev/null || true
    echo "✓ All services stopped"
    exit 0
}
trap cleanup SIGINT SIGTERM

echo ""
echo "═══ Quilt React Dev Environment ═══"
echo ""

# ── 1. API Key Setup ───────────────────────────────────────────────
echo "▶ API key setup..."
if [ -f "$ENV_FILE" ] && grep -q '^VITE_QUILT_API_KEY=' "$ENV_FILE" 2>/dev/null; then
    API_KEY=$(grep '^VITE_QUILT_API_KEY=' "$ENV_FILE" | head -1 | cut -d= -f2-)
    API_KEY=$(echo "$API_KEY" | xargs)
    if [ -n "$API_KEY" ]; then
        echo "  ✓ Using existing API key from $ENV_FILE"
    fi
fi

if [ -z "${API_KEY:-}" ]; then
    if command -v uuidgen > /dev/null 2>&1; then
        API_KEY=$(uuidgen | tr '[:upper:]' '[:lower:]')
    else
        API_KEY=$(cat /proc/sys/kernel/random/uuid 2>/dev/null || \
                  python3 -c 'import uuid; print(uuid.uuid4())' 2>/dev/null || \
                  uuidgen 2>/dev/null | tr '[:upper:]' '[:lower:]')
    fi
    echo "VITE_QUILT_API_KEY=$API_KEY" > "$ENV_FILE"
    echo "  ✓ Generated new API key → $ENV_FILE"
fi

export QUILT_API_KEY="$API_KEY"
echo "  API key: ${API_KEY:0:8}..."

# ── 2. Build backend (debug) ─────────────────────────────────────
echo ""
echo "▶ Building backend..."
cargo build -p quilt-server 2>&1 | tail -3
echo "  ✓ Backend built"

# ── 3. Build WASM (initial) ──────────────────────────────────────
echo ""
echo "▶ Building WASM..."
wasm-pack build "$ROOT/crates/quilt-core" --target web --out-dir "$WASM_PKG_DIR" --dev 2>&1 | tail -3
echo "  ✓ WASM built"

# ── 4. Initial frontend build + sync into wasm_assets ───────────
# This guarantees the server (port 3737) and any container/quadlet
# deployment always serves the latest frontend, even before the
# Node watcher kicks in.
echo ""
echo "▶ Initial frontend build (so 3737 also serves the latest UI)..."
cd "$ROOT/quilt-ui"
npx vite build 2>&1 | tail -5
bash "$ROOT/scripts/sync-frontend-assets.sh" 2>&1 | tail -3
echo "  ✓ Initial bundle synced to wasm_assets/"

# ── 5. Install deps ───────────────────────────────────────────────
echo ""
echo "▶ Checking React deps..."
npm install --silent 2>/dev/null || true
echo "  ✓ Deps ready"

# ── 6. Start backend ───────────────────────────────────────────────
cd "$ROOT"
echo ""
echo "▶ Starting services..."
mkdir -p "$QUILT_DIR"
QUILT_GRAPH_DIR="$QUILT_DIR" \
    QUILT_CORS=true \
    QUILT_API_KEY="$API_KEY" \
    RUST_LOG=quilt_server=info \
    "$ROOT/target/debug/quilt-server" > "$QUILT_DIR/server.log" 2>&1 &
BACKEND_PID=$!
sleep 2
if curl -sf http://localhost:$SERVER_PORT/health > /dev/null 2>&1; then
    echo "  ✓ Backend  — http://localhost:$SERVER_PORT (PID $BACKEND_PID)"
else
    echo "  ✗ Backend failed. Check: tail -f $QUILT_DIR/server.log"
    exit 1
fi

# ── 7. Start WASM watcher (Rust → JS) ─────────────────────────────
cargo watch \
    --watch "$ROOT/crates/quilt-core/src" \
    --watch "$ROOT/crates/quilt-core/Cargo.toml" \
    --shell "wasm-pack build crates/quilt-core --target web --out-dir '$WASM_PKG_DIR' --dev 2>/dev/null && echo '✅ WASM rebuilt' || echo '❌ WASM build failed'" \
    > "$QUILT_DIR/wasm-watch.log" 2>&1 &
WATCH_PID=$!
echo "  ✓ WASM watcher — auto-rebuild on Rust changes (PID $WATCH_PID)"

# ── 8. Start Node-based asset watcher (frontend → wasm_assets) ────
# Watches quilt-ui/src/ for changes and triggers a fresh `vite build` +
# sync to wasm_assets/, so 3737 always serves the latest UI.
node "$ROOT/scripts/frontend-asset-watcher.mjs" > "$QUILT_DIR/asset-watch.log" 2>&1 &
ASSET_WATCH_PID=$!
echo "  ✓ Asset watcher — auto-rebuild + sync to wasm_assets/ (PID $ASSET_WATCH_PID)"

# ── 9. Start Vite dev server (HMR for React) ──────────────────────
echo ""
echo "═══ Two URLs — open the one you want ═══"
echo ""
echo "  ★ http://localhost:$REACT_PORT   — DEV with HMR (preferred)"
echo "    http://localhost:$SERVER_PORT — API + auto-rebuilt bundle"
echo ""
echo "  Ctrl+C to stop all services"
echo ""

# Vite is the foreground process — it owns the terminal.
cd "$ROOT/quilt-ui"
npx vite --port $REACT_PORT

# If vite exits normally, cleanup
cleanup
