# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased] — 2026-06-17

### Changed
- **Graph Space bootstrap unification (Slice A, ADR-0030)**: server, CLI
  and MCP now share a single canonical bootstrap path
  (`quilt_platform::init::init_graph`) that resolves the Graph Space at
  `<graph-root>/.quilt/quilt.db`. The local `ensure_vault_exists` in
  `quilt-server` is removed. The orphaned `settings_repo.rs` in
  `quilt-infrastructure` is deleted (active impl lives in
  `repositories.rs`).

### Deprecated
- `--db-path` CLI flag and `QUILT_DB_PATH` env var are deprecated in
  favor of `--graph-dir` / `QUILT_GRAPH_DIR`. Old names still work with
  a deprecation warning to stderr and will be removed in the next
  minor release. See ADR-0030.
- `QUILT_VAULT_PATH` env var (server) is deprecated in favor of
  `QUILT_GRAPH_DIR`. Old name still works with a deprecation warning.
- `VaultConfig`, `VaultError`, `ensure_vault_exists`, `init_vault`
  symbols in `quilt_platform::init` are deprecated; use the `Graph*`
  equivalents.

### Fixed
- **Playwright E2E testDir mismatch** (`playwright.config.ts:16`):
  changed `testDir: './tests/e2e'` to `testDir: './e2e'` so the
  existing 5 E2E specs in `./e2e/` are actually picked up by
  `just test-e2e`.

## [Unreleased] — 2026-06-01

### Changed
- **UI stack migration**: Leptos 0.8 → React 19 + TypeScript (ADR-0005 updated)
- **Editor engine**: CodeMirror 6 → TipTap (ProseMirror-based) (ADR-0007 updated)
- **MCP tool prefix**: `logseq_*` → `quilt_*` per ADR-0001

### Added
- Bearer token authentication on all `/api/v1/*` endpoints
- Multi-block selection with Alt+Up/Down
- Tabs system with Ctrl+T/Ctrl+W
- Graph view, search modal, backlinks, properties, autocomplete, slash commands

## [0.1.0] - 2026-05-21

### Added
- Core editor with BlockSegment content model
- MCP server with 30+ tools
- SQLite-based local storage
- FTS5 full-text search
- Cognitive AI engines (Serendipity, TreeRAG, etc.)
- Leptos-based WASM UI with dark mode
- Plugin architecture (static)
- Accessibility improvements (WCAG AA)
- Multiple graph file support (UI infrastructure)
- E2E tests with Playwright

### Features
- Journal, Pages, Search, Query views
- Graph visualization
- Cognitive dashboard
- Right sidebar with properties/annotations/backlinks
- Theme toggle (light/dark)
- Toast notifications
- Error boundaries

## [0.0.1] - 2026-05-02

### Added
- Initial project structure
- Domain entities
- Infrastructure scaffolding
