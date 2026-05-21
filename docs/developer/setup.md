# Developer Setup

## Prerequisites

- Rust 1.75+ (use `rustup update`)
- wasm-pack: `cargo install wasm-pack`
- Trunk: `cargo install trunk`
- Node.js 18+ for WASM builds
- Just: `cargo install just`

## Building

```bash
# Build entire workspace
cargo build --workspace

# Build UI only (WASM)
cd crates/quilt-ui
wasm-pack build --target web

# Run UI dev server
cd crates/quilt-ui
trunk serve
```

## Running Tests

```bash
# Unit tests
cargo test --workspace

# UI tests
cargo test -p quilt-ui

# Integration tests
cargo test -p quilt-integration-tests

# E2E tests (requires dev server)
just e2e-install
just e2e-test
```

## IDE Setup

### VS Code
Recommended extensions:
- rust-analyzer
- LLDB debug adapter

### Rust Analyzer
Ensure `rust-analyzer.checkOnSave.command = "clippy"` in settings.
