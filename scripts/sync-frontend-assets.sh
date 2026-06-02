#!/usr/bin/env bash
# sync-frontend-assets.sh — Copy the freshly-built React SPA into the
# Rust server's embedded asset directory.
#
# The Axum server reads from `crates/quilt-server/wasm_assets/` on every
# request (not embedded into the binary). So once you've rebuilt the
# frontend (`npx vite build` or `just react-build`), you must sync the
# output here for the server to serve the latest version.
#
# Usage:
#   bash scripts/sync-frontend-assets.sh
#
# Exits non-zero if the source dist directory is missing.

set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="$ROOT/quilt-ui/dist"
DEST="$ROOT/crates/quilt-server/wasm_assets"

if [ ! -d "$SRC" ]; then
    echo "✗ $SRC does not exist. Run 'npx vite build' in quilt-ui/ first." >&2
    exit 1
fi

# index.html at the root of dist — copy it to the destination root too.
mkdir -p "$DEST"

# Copy index.html
if [ -f "$SRC/index.html" ]; then
    cp "$SRC/index.html" "$DEST/index.html"
fi

# Copy the assets/ directory (chunked JS, CSS, WASM, etc.).
# Use rsync if available to avoid copying the entire tree when only a
# few files changed; fall back to cp -r.
if command -v rsync > /dev/null 2>&1; then
    rsync -a --delete "$SRC/assets/" "$DEST/assets/"
else
    rm -rf "$DEST/assets"
    cp -r "$SRC/assets" "$DEST/assets"
fi

# Surface what was copied so the user can see the change is live.
echo "✓ Synced $SRC → $DEST"
echo ""
echo "Files in wasm_assets/:"
ls -1 "$DEST" | head -10
echo "  ..."
echo ""
echo "Mtime of index.html: $(stat -c %y "$DEST/index.html" 2>/dev/null || stat -f %Sm "$DEST/index.html")"
