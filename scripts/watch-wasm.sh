#!/usr/bin/env bash
# Watch quilt-core Rust source changes and rebuild WASM automatically.
# Uses cargo-watch to detect changes and wasm-pack to rebuild.
#
# Usage: scripts/watch-wasm.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT_DIR="$ROOT/quilt-ui/src/core/wasm-bridge/pkg"

echo "👀 Watching quilt-core/ for changes..."
echo "   WASM output → $OUT_DIR"
echo ""

cargo watch \
    --watch crates/quilt-core/src \
    --watch crates/quilt-core/Cargo.toml \
    --shell "wasm-pack build crates/quilt-core --target web --out-dir '$OUT_DIR' --dev 2>&1 && echo '✅ WASM rebuilt' || echo '❌ Build failed'"
