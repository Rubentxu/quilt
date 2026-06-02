#!/usr/bin/env bash
# Ensure a stable API key exists and print it.
# Used by the justfile recipes (server-start, dev-fast, etc.)
#
# If quilt-ui/.env already has VITE_QUILT_API_KEY, print it.
# Otherwise, generate a new UUID, write it to .env, and print it.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ENV_FILE="$ROOT/quilt-ui/.env"

if [ -f "$ENV_FILE" ] && grep -q '^VITE_QUILT_API_KEY=' "$ENV_FILE" 2>/dev/null; then
    KEY=$(grep '^VITE_QUILT_API_KEY=' "$ENV_FILE" | head -1 | cut -d= -f2 | xargs)
    if [ -n "$KEY" ]; then
        echo "$KEY"
        exit 0
    fi
fi

# Generate a new key
KEY=$(cat /proc/sys/kernel/random/uuid 2>/dev/null || \
      python3 -c 'import uuid; print(uuid.uuid4())' 2>/dev/null || \
      uuidgen 2>/dev/null | tr '[:upper:]' '[:lower:]')

echo "VITE_QUILT_API_KEY=$KEY" > "$ENV_FILE"
echo "  ✓ Generated API key → $ENV_FILE" >&2
echo "$KEY"
