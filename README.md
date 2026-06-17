# Quilt — AI-first Knowledge Graph

[![CI](https://github.com/rubentxu74/quilt/actions/workflows/ci.yml/badge.svg)](https://github.com/rubentxu74/quilt/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/rubentxu74/quilt/branch/main/graph/badge.svg)](https://codecov.io/gh/rubentxu74/quilt)

> **Status:** Production Ready MVP — Week 12 Complete
>
> Rust reimplementation of the Logseq DB graph model with MCP-first architecture.

## Features

- **Knowledge Graph**: Blocks and pages with bidirectional linking
- **Query DSL**: Powerful query language for searching and filtering
- **Full-Text Search**: FTS5-powered search with snippet extraction
- **MCP Server**: Model Context Protocol server for AI agent integration
- **Cognitive AI**: Engines for knowledge discovery and analysis
- **Local-First**: SQLite-based storage with sync-ready architecture

## Tech Stack

| Component | Technology |
|-----------|------------|
| Language | Rust 2024 |
| Database | SQLite + sqlx |
| Search | FTS5 |
| Sync | LWW CRDT (ready for integration) |
| Desktop | Tauri 2 |
| Agent Protocol | MCP |

## Quick Start

```bash
# Initialize a new graph (Graph Space, ADR-0030)
quilt --graph-dir /path/to/my-graph init my-graph

# Open an existing graph
quilt --graph-dir /path/to/my-graph open

# Create a page
quilt --graph-dir /path/to/my-graph page "My Notes"

# Create a block
quilt --graph-dir /path/to/my-graph block --page "My Notes" "A task to do"

# Search content
quilt --graph-dir /path/to/my-graph search "task"

# Execute a query
quilt --graph-dir /path/to/my-graph query "(task todo)"

# Create journal entry
quilt --graph-dir /path/to/my-graph journal

# Start MCP server
quilt --graph-dir /path/to/my-graph serve
```

> The `--db-path` flag and `QUILT_DB_PATH` env var are deprecated;
> use `--graph-dir` / `QUILT_GRAPH_DIR` (see [Graph Space](#graph-space-adr-0030) below).

## Graph Space (ADR-0030)

Quilt operates on a **Graph Space** model: a user-chosen directory
(`<graph-root>`) hosts Quilt's canonical persistence under
`<graph-root>/.quilt/quilt.db`. A Graph Space is the unit the user
creates, opens, closes and switches between.

| Surface | Canonical | Deprecated |
| --- | --- | --- |
| CLI flag | `--graph-dir` | `--db-path` (one-release window) |
| Server env | `QUILT_GRAPH_DIR` | `QUILT_VAULT_PATH` |
| MCP env | `QUILT_GRAPH_DIR` | `QUILT_DB_PATH` |

The canonical bootstrap lives in `quilt_platform::init::init_graph` and
is shared by server, CLI and MCP. See
[`docs/adr/0030-graph-space-journal-first-lifecycle.md`](docs/adr/0030-graph-space-journal-first-lifecycle.md)
and [`docs/graph-space-migration-plan.md`](docs/graph-space-migration-plan.md).

## CLI Commands

| Command | Description |
|---------|-------------|
| `init` | Initialize a new graph database |
| `open` | Open an existing graph |
| `page` | Create a new page |
| `block` | Create a new block |
| `journal` | Create a journal page for today |
| `query` | Execute a query |
| `search` | Search across all content |
| `serve` | Start the MCP server |
| `list-pages` | List all pages |
| `page-info` | Get page info |

## Query Language

Quilt supports a powerful query DSL:

```
(task todo)                    # Find all todo blocks
(priority a)                  # Find blocks with priority A
(and (task todo) (priority a))  # Intersection
(or (task todo) (task done))  # Union
(not (task done))             # Negation
(page "Page Name")            # Blocks on a specific page
[[Page Reference]]            # Blocks referencing a page
(full-text-search "query")    # FTS5 search
```

## MCP Tools

The MCP server provides these tools:

### Page Tools
- `create_page` - Create a new page
- `get_page` - Get page by name or ID
- `list_pages` - List all pages
- `delete_page` - Delete a page

### Block Tools
- `create_block` - Create a new block
- `get_block` - Get block by ID
- `update_block` - Update a block
- `delete_block` - Delete a block
- `move_block` - Move a block to a new parent/position

### Search Tools
- `search` - Full-text search
- `query` - Execute query DSL

### Journal Tools
- `get_journal_today` - Get today's journal page
- `create_journal_entry` - Create entry on today's journal

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        CLI / Tauri UI                       │
├─────────────────────────────────────────────────────────────┤
│                     quilt-application                       │
│                  (Query Service, etc.)                      │
├─────────────────────────────────────────────────────────────┤
│  quilt-domain  │  quilt-query  │  quilt-search  │  quilt-mcp │
│  (Entities)    │  (Parser,     │  (FTS5)        │  (Server)   │
│                │   Executor)   │                │             │
├─────────────────────────────────────────────────────────────┤
│                   quilt-infrastructure                      │
│              (SQLite Repositories, etc.)                     │
├─────────────────────────────────────────────────────────────┤
│                        SQLite DB                            │
└─────────────────────────────────────────────────────────────┘
```

## Performance

| Metric | Target | Status |
|--------|--------|--------|
| Query P95 | < 100ms | ✅ |
| Search P95 | < 100ms | ✅ |
| Startup | < 2s | ✅ |
| Binary size | < 50MB | ✅ (7.1MB) |

## Development

```bash
# Build
cargo build --release

# Run tests
cargo test

# Run clippy
cargo clippy -- -D warnings

# Run benchmarks
cargo bench --package quilt-benchmarks
```

## Documentation

All reverse engineering and design documents live in `docs/reversa/`:

| Document | Description |
|----------|-------------|
| `rust-reimplementation-proposal.md` | Full Rust reimplementation proposal |
| `rust-mcp-ai-deep-dive.md` | Architecture deep dive with code samples |
| `rust-properties-classes-petgraph-eval.md` | Properties + classes system + petgraph evaluation |
| `quilt-mcp-agent-capabilities.md` | 7 MCP agent capabilities design |
| `quilt-ui-workflows.md` | UI views, workflows, user experience |
| `_reversa_sdd/` | SDD formal specs and analysis |

## License

MIT OR Apache-2.0
