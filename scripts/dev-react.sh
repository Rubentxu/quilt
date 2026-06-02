#!/usr/bin/env bash
# Quilt React dev environment — starts all services with auto auth setup.
# Usage: just dev  OR  scripts/dev-react.sh
#
# Starts:
#   1. API key setup (auto-generate if missing, sync to .env)
#   2. quilt-server (backend, port 3737) — with QUILT_API_KEY
#   3. cargo-watch (WASM rebuild on Rust changes)
#   4. vite (React dev server, port 5173) — reads VITE_QUILT_API_KEY from .env
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
# Ensure a consistent API key exists so the server and frontend agree.
# Once generated, the key is stable (persisted in quilt-ui/.env).
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
                  python -c 'import uuid; print(uuid.uuid4())' 2>/dev/null)
    fi
    echo "VITE_QUILT_API_KEY=$API_KEY" > "$ENV_FILE"
    echo "  ✓ Generated new API key → $ENV_FILE"
fi

export QUILT_API_KEY="$API_KEY"
echo "  API key: ${API_KEY:0:8}..."
echo ""

# ── 2. Build backend ───────────────────────────────────────────────
echo "▶ Building backend..."
cargo build -p quilt-server 2>&1 | tail -3
echo "  ✓ Backend built"

# ── 3. Build WASM ──────────────────────────────────────────────────
echo "▶ Building WASM..."
wasm-pack build "$ROOT/crates/quilt-core" --target web --out-dir "$WASM_PKG_DIR" --dev 2>&1 | tail -3
echo "  ✓ WASM built"

# ── 4. Install deps ────────────────────────────────────────────────
echo "▶ Checking React deps..."
cd "$ROOT/quilt-ui"
npm install --silent 2>/dev/null || true
echo "  ✓ Deps ready"

# ── 5. Start backend ───────────────────────────────────────────────
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

# ── 6. Start WASM watcher ──────────────────────────────────────────
cargo watch \
    --watch "$ROOT/crates/quilt-core/src" \
    --watch "$ROOT/crates/quilt-core/Cargo.toml" \
    --shell "wasm-pack build crates/quilt-core --target web --out-dir '$WASM_PKG_DIR' --dev 2>/dev/null && echo '✅ WASM rebuilt' || echo '❌ WASM build failed'" \
    > "$QUILT_DIR/wasm-watch.log" 2>&1 &
WATCH_PID=$!
echo "  ✓ WASM watcher — auto-rebuild on Rust changes (PID $WATCH_PID)"

# ── 7. Start Vite ──────────────────────────────────────────────────
echo "  ▶ React    — http://localhost:$REACT_PORT"
echo ""
echo "  Ctrl+C to stop all services"
echo ""

npx vite --port $REACT_PORT

# If vite exits normally, cleanup
cleanup
