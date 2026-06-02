# Quilt — AI Agent Instructions

## Project identity
- **Name:** Quilt — AI-first Knowledge Graph
- **Stack:** Rust (2021 edition), SQLite, Tokio, Axum, React 18 (TypeScript), Vite, WASM
- **Architecture:** Clean/Hexagonal (Domain → Application → Infrastructure → Presentation)
- **Origin:** Inherits Logseq DB graph model, ground-up Rust reimplementation
- **Docs:** `docs/reversa/` reverse engineering analysis

## Principles
1. MCP-first architecture — all operations are MCP tools
2. Type safety — properties are typed (not strings in frontmatter)
3. AI agents are first-class users, not afterthought
4. Zero panics in runtime — every error path is handled
5. WASM target compatibility — quilt-core compiles to wasm32-unknown-unknown

---

## Architecture (Clean — dependencies point inward)

```
quilt-ui/ (React + Vite, port 5173)
    │ api-client.ts  ──Bearer token──▶  quilt-server (Axum, port 3737)
    │                                    │ auth middleware
    │                                    ├─ /api/v1/blocks, /pages, /search
    │                                    ├─ /ws (WebSocket)
    │                                    └─ /* (SPA assets)
    ▼
quilt-core (WASM) ◀── loaded by quilt-ui
```

### Crate layers

| Layer | Crates |
|-------|--------|
| Presentation | quilt-server, quilt-bin |
| Application | quilt-application, quilt-mcp |
| Domain | quilt-domain, quilt-query, quilt-search, quilt-core |
| Infrastructure | quilt-infrastructure |
| Desktop | quilt-platform (Tauri) |
| Cognitive | quilt-analysis |
| Test support | quilt-test-helpers |

| Crate | Responsibility |
|-------|---------------|
| `quilt-domain` | Entities, Value Objects, Repository traits, DomainError |
| `quilt-application` | Use cases, Service interfaces, DTOs, Commands |
| `quilt-infrastructure` | SQLite pool, SqliteBlockRepo, InMemoryRepo |
| `quilt-core` | Outliner, parser, graph layout, CRDT sync (WASM) |
| `quilt-search` | FTS5 query builder, sanitization |
| `quilt-query` | DSL parser (pest), AST, query execution |
| `quilt-mcp` | MCP protocol server, resource/block/page handlers |
| `quilt-server` | Axum HTTP, REST handlers, WebSocket, auth middleware |
| `quilt-platform` | Tauri shell |
| `quilt-bin` | CLI (clap) |
| `quilt-analysis` | Agent memory, structural analysis |
| `quilt-test-helpers` | InMemoryBlockRepo, InMemoryPageRepo, factories |

---

## Frontend (quilt-ui/) — target structure

```
src/
├── core/
│   ├── api-client.ts          # Single HTTP client — Bearer auth via VITE_QUILT_API_KEY
│   └── wasm-bridge/           # WASM loader + React context
├── features/                  # Feature-based
│   ├── outliner-tiptap/       # Block editor (TipTap)
│   ├── properties/            # Block property panel
│   ├── search/                # Search modal, autocomplete
│   ├── references/            # Backlinks panel
│   ├── comments/              # Comment threads
│   ├── cognitive/             # Agent activity panel
│   └── sidebar/               # Navigation sidebar
├── pages/                     # Route-level page components
├── shared/
│   ├── components/            # AppShell, ErrorBoundary, Skeleton, Tabs
│   ├── contexts/              # ConnectionContext, TabsContext
│   ├── hooks/                 # useSSE, usePollingSync, useResponsive, useBlockHistory
│   ├── types/api.ts           # TypeScript API types
│   └── utils/                 # blockProperties, formatJournalDate
├── test/setup.ts              # Vitest global setup
├── App.tsx / router.tsx / main.tsx
```

**Tech**: React 18, TypeScript, Vite, TipTap, Tailwind CSS, React Router, TanStack Virtual, dnd-kit, vitest, @testing-library/react

---

## Auth flow (CRITICAL)

```
1. Server start (main.rs:106-118):
   - QUILT_API_KEY env set → use it
   - Not set → auto-generate UUID v4, print to console
   - middleware::auth::init(key) stores in OnceLock

2. Frontend setup:
   - Copy generated key from server console
   - quilt-ui/.env:  VITE_QUILT_API_KEY=<key>
   - Vite injects at build time → import.meta.env.VITE_QUILT_API_KEY
   - api-client.ts adds Authorization: Bearer <key> to every request

3. Every API call:
   - Must include Authorization: Bearer <key>
   - auth.rs middleware checks against OnceLock
   - Missing/wrong → 401 Unauthorized
```

**Protected**: `/api/v1/*` (blocks, pages, search, settings, navigate)
**Public**: `/health`, `/metrics`, `/ws`, `/`, `/*path` (SPA assets)

---

## Commands

### Rust
| Command | Purpose |
|---------|---------|
| `cargo build` | Compile workspace |
| `cargo test` | Run all Rust tests |
| `cargo test -p <crate>` | One crate |
| `cargo fmt` | Format |
| `cargo clippy` | Lint |
| `cargo run -p quilt-server` | Start server (prints API key!) |

### Frontend
| `cd quilt-ui && npm run dev` | Vite (5173) |
| `cd quilt-ui && npm run build` | Production → wasm_assets/ |
| `cd quilt-ui && npx vitest run` | Component tests |

### E2E
| `QUILT_API_KEY=<key> npx playwright test` | All E2E |
| `npx playwright test --headed --project=chromium` | Debug |
| `npx playwright test --grep @smoke` | Smoke only |

### Justfile
| `just check` | Fast compile check |
| `just ci` | Format + clippy + tests |
| `just test-e2e` | E2E with proper setup |

---

## Conventions

### Rust
- `Result<T, E>` recoverable; `panic!` only for unrecoverable — never in runtime code
- `thiserror` for libraries, `anyhow` for binaries
- `sqlx` (async, compile-time checked queries)
- Models in `src/models/`, DB in `src/db/` (per crate)
- `#[tracing::instrument]` on public functions
- `///` on all public API
- Unit tests: `#[cfg(test)]` module inline
- Integration tests: `tests/` directory per crate

### TypeScript / React
- Feature-based: `features/<name>/`
- Shared code in `shared/`
- Tests in `__tests__/` colocated
- All API calls through `api-client.ts` — never raw fetch
- Tailwind utility classes + shared globals.css

### Tests (hard rules — non-negotiable)
- **Test behavior, not implementation** — no `mock_called()` assertions
- **Smallest layer first** — unit before integration before E2E
- **Deterministic** — no real `now()`, no real network, inject clocks/mocks
- **One runtime per async test** — never share tokio Runtime
- **Regression test for every bug** — write first, watch it fail, then fix
- **E2E tests MUST FAIL, not skip** — NEVER `try/catch` + `test.skip()`. If API fails, test fails.
- **E2E auth required** — every API call must include `Authorization: Bearer` from `QUILT_API_KEY`
- **No `waitForTimeout`** — use `findBy*`, `expect().toBeVisible()`, `waitForSelector`
- **No CSS selectors** — `getByRole`/`getByLabelText`/`getByText`; `getByTestId` last resort
- **Property tests** (`proptest`) for parsers, serializers, invariants

### Test pyramid

```
        ╱───────╲
       ╱   E2E   ╲          Playwright (tests/e2e/spec/)
      ╱───────────╲         Needs running server + frontend + QUILT_API_KEY
     ╱  Component  ╲        Vitest + Testing Library (quilt-ui/src/__tests__/)
    ╱───────────────╲
   ╱   Integration   ╲      crate/tests/*.rs (InMemory repos)
  ╱───────────────────╲
 ╱       Unit          ╲     #[cfg(test)] next to code
╱───────────────────────╲
```

---

## E2E test infrastructure (target design)

```
tests/e2e/
├── auth-state.ts              # Reads QUILT_API_KEY → provides auth headers
├── quilt.spec.ts              # P0 baseline (sidebar nav, search, query, cognitive)
└── spec/
    ├── journal.spec.ts        # Journal: navigation, blocks, prev/next
    ├── page-editing.spec.ts   # Edit blocks, Enter split, persistence
    ├── outliner.spec.ts       # Keyboard navigation, indent/outdent
    ├── search.spec.ts         # Search + autocomplete
    ├── navigation.spec.ts     # SPA routing
    ├── smoke.spec.ts          # @smoke critical path
    ├── error-handling.spec.ts # Offline, 404, retry
    ├── accessibility.spec.ts  # a11y audit (axe-core)
    └── visual-regression.spec.ts # Screenshot comparison
```

### Auth in E2E specs (mandatory pattern)

```typescript
import { getAuthHeaders } from '../auth-state';

// Every API call:
const headers = getAuthHeaders();
await page.request.post('/api/v1/blocks', { data, headers });
```

### E2E test must fail, never skip
- ❌ `try { ... } catch { test.skip(...) }` — masks failures
- ✅ Assert the real response. If backend unavailable, fail in global setup, not per-test.
- ✅ Use `test.fail()` only for known-broken things, with an issue link.

---

## Key references
- `docs/reversa/` — Reverse engineering analysis
- `docs/test-strategy.md` — Test strategy and pyramid roadmap
- `docs/AUDIT_REPORT.md` — Full audit report
- `docs/COVERAGE.md` — Coverage report
- `docs/PROPERTY_TESTS.md` — Property test guide
- `docs/PERFORMANCE.md` — Performance benchmarks
- `CONTEXT.md` — Domain language and concepts
- `DESIGN.md` — Design system
- `openspec/` — SDD specs and proposals
