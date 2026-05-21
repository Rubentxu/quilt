#!/bin/bash
# Optimize WASM bundle size using wasm-opt
#
# Usage:
#   ./scripts/optimize-wasm.sh <input.wasm> [output.wasm]
#
# Requires: wasm-opt from binaryen package

set -e

INPUT_WASM="${1:-pkg/quilt_ui_bg.wasm}"
OUTPUT_WASM="${2:-pkg/quilt_ui_opt.wasm}"

if [ ! -f "$INPUT_WASM" ]; then
    echo "Error: Input WASM file not found: $INPUT_WASM"
    echo "Run 'wasm-pack build --target web' first"
    exit 1
fi

# Check if wasm-opt is available
if ! command -v wasm-opt &> /dev/null; then
    echo "Warning: wasm-opt not found. Installing binaryen..."
    # Try to install via cargo
    if command -v cargo &> /dev/null; then
        cargo install binaryen 2>/dev/null || {
            echo "Error: Could not install binaryen."
            echo "On macOS: brew install binaryen"
            echo "On Ubuntu/Debian: apt install binaryen"
            echo "On Fedora: dnf install binaryen"
            exit 1
        }
    else
        echo "Error: cargo not found. Cannot install binaryen automatically."
        exit 1
    fi
fi

echo "Optimizing WASM: $INPUT_WASM -> $OUTPUT_WASM"

# Optimize with -Oz (optimize for size)
wasm-opt -Oz -o "$OUTPUT_WASM" "$INPUT_WASM"

INPUT_SIZE=$(stat -c%s "$INPUT_WASM")
OUTPUT_SIZE=$(stat -c%s "$OUTPUT_WASM")
REDUCTION=$((100 - (OUTPUT_SIZE * 100 / INPUT_SIZE)))

echo ""
echo "Size reduction: $REDUCTION%"
echo "  Before: $((INPUT_SIZE / 1024))KB"
echo "  After:  $((OUTPUT_SIZE / 1024))KB"

# If optimized is smaller, replace original
if [ $OUTPUT_SIZE -lt $INPUT_SIZE ]; then
    mv "$OUTPUT_WASM" "$INPUT_WASM"
    echo "Replaced original with optimized version."
else
    echo "Optimized version is not smaller. Keeping original."
    rm -f "$OUTPUT_WASM"
fi
