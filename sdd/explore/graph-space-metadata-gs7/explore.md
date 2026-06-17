# Kernel Exploration: GS-7 — Graph Space metadata (name, icon, description, color, created_at) inside canonical SQLite

> Source: `sddk/graph-space-metadata-gs7/explore`
> Date: 2026-06-17
> Topic: Phase 7 of `docs/graph-space-migration-plan.md` (ADR-0030 §17)

## Current State

### What GS-7 must deliver (per ADR-0030 §17 and the migration plan)

ADR-0030 §17 enumerates the fields and the location rule:

> "La metadata del Graph Space incluye: nombre, icono, descripción, color/tema, path, fecha de creación. Esa metadata vive dentro del propio Graph y se edita desde la configuración del Graph, no desde el selector."

So the canonical home is `<graph-root>/.quilt/quilt.db` (i.e., the per-graph `quilt.db` created by `init_graph` in `crates/quilt-platform/src/init.rs:97`), not the cross-graph `~/.local/share/quilt/global.db` (`crates/quilt-infrastructure/migrations/global/0001_init.sql`).

`docs/graph-space-migration-plan.md` lines 146-169 already lists Phase 7 as a distinct work item and **recommends a dedicated table** rather than overloading `config`:

> "Crear tabla dedicada en SQLite en lugar de sobrecargar `config`."
> Archivos candidatos:
> - migraciones SQLite
> - repositorio de settings o nuevo `graph_space_repo`
> - UI de configuración del Graph

### What the schema already has (audit of `migrations/` + `connection.rs`)

`crates/quilt-infrastructure/migrations/001_initial_schema.sql` creates two generic key-value tables:

```sql
CREATE TABLE kv_store (
    key TEXT PRIMARY KEY NOT NULL,
    value BLOB NOT NULL,
    updated_at INTEGER NOT NULL
);
CREATE TABLE config (
    key TEXT PRIMARY KEY NOT NULL,
    value BLOB NOT NULL,
    updated_at INTEGER NOT NULL
);
```

A grep across the workspace for `kv_store`, `config` (as table), and `ConfigRepository` finds **zero readers or writers**. Both tables are dead weight — created by `run_migrations` in `crates/quilt-infrastructure/src/database/sqlite/connection.rs:359-393` but never queried. The only matches are in the doc comment of `connection.rs` (line 92-94) listing them as available surfaces. They are not represented in `quilt-domain` (no `KvRepository` / `ConfigRepository` trait) and there is no Rust struct for their rows.

The only typed singleton table that exists today for per-graph state is `user_settings` (migration 003 + inlined in `connection.rs:472`):

```sql
CREATE TABLE user_settings (
    id INTEGER PRIMARY KEY DEFAULT 1 CHECK (id = 1),
    timezone TEXT NOT NULL DEFAULT 'UTC',
    journal_format TEXT NOT NULL DEFAULT '%Y-%m-%d',
    start_of_week INTEGER NOT NULL DEFAULT 1 CHECK (start_of_week BETWEEN 0 AND 6),
    preferred_format TEXT NOT NULL DEFAULT 'markdown' CHECK(...),
    updated_at INTEGER NOT NULL
);
```

Pattern is **singleton** (`CHECK id = 1`), typed columns (no JSON blob), explicit `INSERT OR IGNORE` bootstrap, exposed through `quilt_domain::entities::UserSettings` + `SettingsRepository` trait (`crates/quilt-domain/src/repositories/settings_repository.rs`).

There is also a typed-table precedent for per-page identity (`pages.name` + `pages.title` in `crates/quilt-domain/src/entities/page.rs:14-43`): the `name` is canonical/uniqueness-scoped, `title` is optional and used for display. The same split maps cleanly to a Graph Space (canonical directory-derived path vs. user-facing display name).

### What the frontend currently shows about graph identity

- `GraphSelectorPage.tsx` (`quilt-ui/src/pages/GraphSelectorPage.tsx:34-38`) derives a display name from the path:
  ```ts
  function formatDate(path: string): string {
    const parts = path.replace(/\\/g, '/').split('/')
    return parts[parts.length - 1] || path
  }
  ```
  i.e. basename only — no `icon`, no `description`, no `color`, no real identity.
- `HomePage.tsx` (`quilt-ui/src/pages/HomePage.tsx:36-42`) only checks whether `lastOpenedGraph` exists; it never reads any metadata.
- `AppShell.tsx`, `Sidebar.tsx`, `JournalPage.tsx`, `GraphViewPage.tsx` — none of them query any graph identity endpoint.
- The API client (`quilt-ui/src/core/api-client.ts:936-1014`) defines Graph Space endpoints only for `getGlobalState`, `setLastOpenedGraph`, `listRecentGraphs`, `createGraph`, `validateGraph`. **No `getGraphMetadata` / `updateGraphMetadata` exists.**
- The MCP server (`crates/quilt-mcp/src/handlers/`) has no `graph` resource either.

In short: the UI today has zero first-class awareness of graph name/icon/description/color. Identity = path basename, full stop.

## Context Quality

- **Level:** C2
- **Evidence present:** ADR-0030 (full §17), migration plan phase 7, all relevant migrations, `user_settings` precedent (entity + repo + sqlite impl + test), `Page.name`/`Page.title` precedent, `GraphSelectorPage` reads, full API client audit, full migrations audit.
- **Missing context:** None blocking. One design choice is open (see Recommendation: singleton row vs. row-per-graph) but the singleton row is consistent with how Quilt boots one graph at a time (ADR-0030 §7: "una sola ventana / instancia activa — un solo Graph activo a la vez").
- **Recommended Effort:** verify (no need to deepen — every adapter surface has been located).

## Knowledge Coverage

| Class | Status | Evidence | Gap Impact |
|------|--------|----------|------------|
| Roadmap/Backlog | present | `docs/graph-space-migration-plan.md:146-169` (Phase 7), `docs/ROADMAP.md:283` | None — Phase 7 is scoped in detail |
| Work Items | partial | `openspec/changes/graph-space-migration-phases-1-4/` covers Phases 1-4 only; **no change folder exists for GS-7** | Need to spin up a new change folder or extend the existing one |
| Architecture/ADRs | present | `docs/adr/0030-graph-space-journal-first-lifecycle.md` (accepted, §17 explicitly) | None |
| Ownership | present | Migration plan names candidate files; `crates/quilt-platform/src/init.rs` owns graph bootstrap | None |
| Learnings | present | `user_settings` singleton pattern is the proven template (`user_settings.rs:34-44` default + validation pattern, `repositories.rs:1647-1704` SQLite impl); `Page.name`/`Page.title` is the proven display-name split | None |

## Problem Taxonomy

| Axis | Applies | Evidence |
|------|---------|----------|
| Domain modeling | Yes | Need a `GraphSpace` (or `GraphMeta`) entity; field semantics (canonical vs display) must follow the `Page.name`/`Page.title` precedent |
| Boundary/seam | Yes | New seam between per-graph DB (`quilt.db`) and cross-graph DB (`global.db`); GS-7 lives in per-graph only |
| Coupling/connascence | Low | No existing callers to couple to; fresh surface |
| API contract | Yes | New endpoints (`GET/PATCH /api/v1/graph/metadata`?) + new MCP resource `graph://{id}` |
| Refactor/legacy | No | Greenfield inside the canonical DB |
| Event/CQRS | No | Singleton, low write volume |
| Testing | Yes | Need `quilt-domain` unit tests (entity), `quilt-infrastructure` integration (sqlite), `quilt-server` HTTP round-trip — same pyramid as `user_settings` |
| Security/operations | Low | Auth gate already covers `/api/v1/*`; no PII, no secrets |

## Domain Language And Invariants

- **Resolved (code):** Graph, GraphConfig, `init_graph`, `validate_graph_layout`, GraphSelectorPage, recent_graphs.
- **Resolved (ADR + plan):** Graph Space (metadata), Graph Content (blocks/pages/links), `created_at`, `path` (derivable), `name`, `icon`, `description`, `color/theme`, Graph Settings UI.
- **Unresolved:** whether `path` should be persisted as a column (the migration plan calls it "derivable, opcionalmente no editable"). Recommendation: persist `path` to detect moves (a moved graph should fail validation loudly per ADR-0030 §6, not silently re-bind).
- **Invariants:**
  - Metadata lives **inside** the graph's `quilt.db` (ADR-0030 §5 — Graph-specific state).
  - One active Graph at a time (ADR-0030 §7) → singleton row is sufficient.
  - Identity must be editable from a Graph Settings surface, **not** the selector (ADR-0030 §17).
  - `path` MUST match the layout the DB was opened from, or the graph is considered moved → fail explicitly (ADR-0030 §6).

## Knowledge Gaps

- No `openspec/changes/<name>/` for GS-7. Either create a new change folder or treat it as an extension of `graph-space-migration-phases-1-4`. The latter is cleaner since the plan already lists Phase 7 in the same sequence.
- No decision on `icon` representation: emoji string vs. library key (`lucide`/`tabler` slug per ADR-0030 §16 "librería curada"). The plan does not pin this; the spec phase must.

## Affected Areas

- `crates/quilt-infrastructure/migrations/` — new file `009_graph_space.sql` (matches the existing `00X_<name>.sql` numbering; current max is `008_annotations.sql`).
- `crates/quilt-infrastructure/src/database/sqlite/connection.rs` — inline the new `CREATE TABLE` so `run_migrations` is the single bootstrap path (mirrors how 008_annotations is inlined).
- `crates/quilt-domain/src/entities/graph_space.rs` — new entity module (parallel to `user_settings.rs`).
- `crates/quilt-domain/src/entities/mod.rs` — `pub use graph_space::GraphSpace;`
- `crates/quilt-domain/src/repositories/graph_space_repository.rs` — new trait (parallel to `settings_repository.rs`).
- `crates/quilt-domain/src/repositories/mod.rs` — `pub use graph_space_repository::GraphSpaceRepository;`
- `crates/quilt-infrastructure/src/database/sqlite/graph_space_repo.rs` — new sqlite impl (parallel to `repositories.rs:1647-1704` SqliteSettingsRepository).
- `crates/quilt-infrastructure/src/database/sqlite/repositories.rs` — re-export.
- `crates/quilt-application/src/use_cases/graph_space.rs` — use case (`get` / `update`).
- `crates/quilt-server/src/handlers/graph_space.rs` — HTTP layer.
- `crates/quilt-server/src/main.rs` + `crates/quilt-bin/src/mcp_main.rs` — wire the new repo (same pattern as `settings_repo`).
- `crates/quilt-mcp/src/handlers/graph.rs` (new) — MCP resource `graph://metadata` for AI agents (MCP-first principle).
- `quilt-ui/src/core/api-client.ts` — add `getGraphMetadata` / `updateGraphMetadata`.
- `quilt-ui/src/pages/GraphSelectorPage.tsx` — replace `formatDate(path)` with `metadata.name || basename`.
- `quilt-ui/src/pages/GraphSettingsPage.tsx` (new) — Graph Settings surface per ADR-0030 §17.
- `quilt-ui/src/router.tsx` — register `/graph/settings`.
- `tests/e2e/spec/graph-metadata.spec.ts` (new) — Playwright E2E.

## Options

| Option | Pros | Cons | Effort |
|--------|------|------|--------|
| **A. Dedicated singleton table `graph_space` (recommended)** | Type-safe columns; matches `user_settings` precedent exactly; CHECK id=1 enforces singleton; SQL constraints per field (icon key whitelist, color hex pattern); idempotent bootstrap via `INSERT OR IGNORE`; clean migration `009_graph_space.sql`; one row per graph fits the one-active-graph invariant | Adds one migration + one entity + one repo + one handler (familiar shape) | M |
| B. Reuse existing `config` table with JSON-encoded blob | Zero migration; fits "already there" argument | Loses type safety on every read; no CHECK constraints (icon whitelist, color format); harder to evolve schema; conflicts with ADR-0030's explicit recommendation in the migration plan | S |
| C. Reuse `kv_store` | Same as B but more "scratch" semantics | Even less typed; `kv_store` is intended for misc state; no migration, no bootstrap | S |
| D. Per-page `pages` row with a reserved `name` like `__graph_space__` | Reuses page pipeline; no new table | Pollutes the page namespace; every page query needs a filter; no typed columns; metadata would inherit page semantics (file_id, journal_day, etc.) it doesn't need | L |

## Entropy Envelope

- **Method:** heuristic (no CogniCode session requested).
- **Coupling risk:** **low**.
  - Singleton row + read-mostly access → trivial SQL, no fan-in.
  - No existing caller to refactor.
  - Frontend hook surface is one selector list and one new settings page.
- **OCP risk:** **low**. Field additions are additive `ALTER TABLE … ADD COLUMN` (the migration runner already supports this pattern at `connection.rs:164-187`, `connection.rs:201-213`, `connection.rs:540-556`).
- **Connascence notes:**
  - **Of name**: the canonical field `name` in this new table will mirror the `Page.name` convention (displayable, not necessarily unique). Acceptable because there is only ever one graph in scope per ADR-0030 §7.
  - **Of value**: `created_at` follows the same `INTEGER NOT NULL` ms epoch convention as every other table (`connection.rs:22-25, 39, 50, 58, 65, 73, 100, 133, …`).
  - **Of algorithm**: the `INSERT OR IGNORE` bootstrap and the idempotent `ALTER TABLE ADD COLUMN` pattern are already in `connection.rs`; no new algorithm.

## Recommendation

**Option A: dedicated `graph_space` singleton table inside `quilt.db`.**

Reasons:
1. The migration plan explicitly recommends it and ADR-0030 §17 enumerates the fields.
2. Mirrors the established `user_settings` pattern end-to-end (entity, repo, sqlite impl, bootstrap). The team already owns that pattern.
3. Singleton row satisfies ADR-0030 §7 (one active graph at a time) without needing per-graph-id scoping.
4. Typed columns + SQL CHECK constraints give us icon/whitelist and color/hex validation at the DB boundary, where it belongs.
5. Reusing the dead `config`/`kv_store` tables would be a step backward (they are untyped, undocumented, and unused for any real purpose).

Design choices the spec phase must lock down:
- `icon`: store as a **library key string** (e.g., `lucide:book-open`) per ADR-0030 §16 "librería curada". Add a CHECK constraint against a known allowlist or rely on app-layer validation.
- `color`: hex string `#RRGGBB` (or `#RRGGBBAA`) with a CHECK `LIKE '#%[0-9A-Fa-f]%'`.
- `path`: persist and validate on read — if mismatch with `GraphConfig.graph_path`, return a typed `GraphMoved` error rather than silently rebinding (per ADR-0030 §6).
- `created_at`: set once at first creation; never updated. Keep `updated_at` for last edit.
- Backfill: in the migration, no rows to backfill (singleton) — `INSERT OR IGNORE` with `created_at = unixepoch() * 1000` is the bootstrap row.

Sequencing inside the change:
1. Migration `009_graph_space.sql` + inline in `connection.rs`.
2. Domain entity + repo trait + sqlite impl + unit + integration tests.
3. Use case `get_graph_space` / `update_graph_space` with validation.
4. HTTP handler `GET /api/v1/graph/metadata` + `PATCH /api/v1/graph/metadata` (auth-gated, same as the rest).
5. MCP resource `graph://metadata` (MCP-first).
6. Frontend api-client, GraphSelectorPage swap, new GraphSettingsPage.
7. E2E: create graph → assert default metadata → edit name → assert persistence after reload.

## Ready For Proposal

**Yes.** Evidence is C2-grade, every adapter surface has been located, and the recommended approach is the same pattern the codebase already uses for `user_settings`. The single open decision (icon representation) belongs in the spec phase, not exploration.
