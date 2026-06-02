#!/usr/bin/env bash
# Quilt React dev environment — starts all services.
# Usage: just dev-react  OR  scripts/dev-react.sh
#
# Starts:
#   1. quilt-server (backend, port 3737)
#   2. cargo-watch (WASM rebuild on Rust changes)
#   3. vite (React dev server, port 1420)
#
# Ctrl+C stops everything cleanly.

set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
QUILT_DIR="${QUILT_DIR:-$HOME/.quilt-dev}"
SERVER_PORT=3737
REACT_PORT=1420
WASM_PKG_DIR="$ROOT/quilt-ui/src/core/wasm-bridge/pkg"

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

# 1. Build backend
echo "▶ Building backend..."
cargo build -p quilt-server 2>&1 | tail -3
echo "  ✓ Backend built"

# 2. Build WASM
echo "▶ Building WASM..."
wasm-pack build "$ROOT/crates/quilt-core" --target web --out-dir "$WASM_PKG_DIR" --dev 2>&1 | tail -3
echo "  ✓ WASM built"

# 3. Install deps
echo "▶ Checking React deps..."
cd "$ROOT/quilt-ui"
npm install --silent 2>/dev/null || true
echo "  ✓ Deps ready"

# 4. Start backend
echo ""
echo "▶ Starting services..."
mkdir -p "$QUILT_DIR"
QUILT_GRAPH_DIR="$QUILT_DIR" QUILT_CORS=true \
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

# 5. Start WASM watcher
cargo watch \
    --watch "$ROOT/crates/quilt-core/src" \
    --watch "$ROOT/crates/quilt-core/Cargo.toml" \
    --shell "wasm-pack build crates/quilt-core --target web --out-dir '$WASM_PKG_DIR' --dev 2>/dev/null && echo '✅ WASM rebuilt' || echo '❌ WASM build failed'" \
    > "$QUILT_DIR/wasm-watch.log" 2>&1 &
WATCH_PID=$!
echo "  ✓ WASM watcher — auto-rebuild on Rust changes (PID $WATCH_PID)"

# 6. Start Vite (foreground)
echo "  ▶ React    — http://localhost:$REACT_PORT"
echo ""
echo "  Ctrl+C to stop all services"
echo ""

npx vite --port $REACT_PORT

# If vite exits normally, cleanup
cleanup
