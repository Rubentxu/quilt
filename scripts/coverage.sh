#!/bin/bash
# Generate coverage report
#
# Produces four artefacts under coverage/:
#   - coverage/summary.txt          Region/function/line summary
#   - coverage.txt                  Per-file text report
#   - coverage/html/index.html      Browsable HTML report
#   - coverage/lcov.info            LCOV report (Codecov, Coveralls, SonarQube)
#
# Requires: cargo-llvm-cov + the llvm-tools rustup component.
#   rustup component add llvm-tools
#   cargo install cargo-llvm-cov

set -euo pipefail

# Run from the repo root regardless of where the script is invoked.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT_DIR"

mkdir -p coverage

echo "=== Coverage summary ==="
cargo llvm-cov --workspace --summary-only --ignore-run-fail | tee coverage/summary.txt

echo ""
echo "=== Per-file coverage (text report) ==="
cargo llvm-cov --workspace --text --output-path coverage.txt --ignore-run-fail

echo ""
echo "=== HTML report at coverage/html/index.html ==="
cargo llvm-cov --workspace --html --output-dir coverage --ignore-run-fail

echo ""
echo "=== LCOV at coverage/lcov.info ==="
cargo llvm-cov --workspace --lcov --output-path coverage/lcov.info --ignore-run-fail

echo ""
echo "Done. Open coverage/html/index.html in a browser to explore."
