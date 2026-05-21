#!/bin/bash
# Check WASM bundle size is within budget

WASM_FILE="pkg/quilt_ui_bg.wasm"
MAX_SIZE=2097152  # 2MB

if [ ! -f "$WASM_FILE" ]; then
    echo "WASM file not found. Run: wasm-pack build --target web"
    exit 1
fi

SIZE=$(stat -f%z "$WASM_FILE")
MAX_GZIP=524288  # 512KB gzipped estimate

echo "WASM size: $((SIZE / 1024))KB"

if [ $SIZE -gt $MAX_SIZE ]; then
    echo "ERROR: WASM bundle exceeds 2MB budget"
    exit 1
fi

echo "WASM bundle size: OK"
