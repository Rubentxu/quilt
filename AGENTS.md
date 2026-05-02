# Quilt — AI Agent Instructions

## Project identity
- **Name:** Quilt — AI-first Knowledge Graph
- **Stack:** Rust (2024 edition), SQLite, Tokio, Tauri
- **Origin:** Inherits Logseq DB graph model, ground-up Rust reimplementation
- **Docs:** `docs/reversa/` contains full reverse engineering analysis

## Principles
1. MCP-first architecture — all operations are MCP tools
2. Type safety — properties are typed (not strings in frontmatter)
3. AI agents are first-class users, not afterthought
4. Zero panics in runtime
5. WASM target compatibility

## Commands
- `cargo build` — Compile
- `cargo test` — Run tests
- `cargo fmt` — Format code
- `cargo clippy` — Lint

## When writing code
- Follow Rust idioms (Result for recoverable, panic only for unrecoverable)
- Use thiserror for error types
- Use sqlx for database (async, compile-time checked queries)
- Models go in `src/models/`, DB logic in `src/db/`
- Add tracing::instrument to public functions
- Add doc comments to public API

## Key references
- `docs/reversa/rust-reimplementation-proposal.md` — Full proposal
- `docs/reversa/rust-mcp-ai-deep-dive.md` — Architecture deep dive
- `docs/reversa/rust-properties-classes-petgraph-eval.md` — Properties + classes detail
- `docs/reversa/erd.md` — Entity relationship diagram
- `docs/reversa/domain.md` — Domain glossary
- `docs/reversa/quilt-mcp-agent-capabilities.md` — MCP agent design
- `docs/reversa/quilt-ui-workflows.md` — UI and workflow design
