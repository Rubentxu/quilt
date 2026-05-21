# Contributing to Quilt

## Development Setup

### Prerequisites
- Rust 1.75+
- wasm-pack
- Node.js 18+
- Just (command runner)

### Quick Start
```bash
# Clone and build
git clone <repo>
cargo build

# Run UI dev server
cd crates/quilt-ui && trunk serve

# Run tests
cargo test --workspace
cargo test -p quilt-ui
```

### Code Standards
- Run `cargo fmt` before committing
- Run `cargo clippy --workspace` to check for lints
- All public APIs must have doc comments
- Use thiserror for error types
- Use Result for recoverable errors

### Commit Convention
We use Conventional Commits:
- `feat: add new search algorithm`
- `fix: resolve memory leak in graph view`
- `docs: update API reference`

### Testing
- Unit tests: `cargo test`
- Integration tests: `cargo test -p quilt-integration-tests`
- E2E tests: `just e2e-install && just e2e-test`

### Pull Request Process
1. Fork and create a branch from `main`
2. Add tests for new functionality
3. Ensure all tests pass
4. Update documentation if needed
5. Request review

## Architecture

See [docs/architecture-ddd.md](docs/architecture-ddd.md) for the full architecture.

## Crate Overview

| Crate | Purpose |
|-------|---------|
| quilt-domain | Core entities and business logic |
| quilt-infrastructure | SQLite repositories |
| quilt-mcp | MCP server and tools |
| quilt-ui | Leptos WASM frontend |
| quilt-cognitive | AI cognitive engines |
