# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

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
