# Kernel Exploration: GS-9 — Manual resource ingestion / reindex from the graph directory (ADR-0030 §4)

> Source: `sddk/manual-resource-ingestion-gs9/explore`
> Date: 2026-06-17
> Topic: Phase 9 of `docs/graph-space-migration-plan.md` and ADR-0030 §4
> Status: **explore — ready for proposal**

---

## Current State

### What GS-9 must deliver (per ADR-0030 §4 and the migration plan)

ADR-0030 §4 (verbatim, ES) defines the only binding contract for GS-9:

> "Si la carpeta del Graph contiene recursos compatibles:
> - Quilt puede detectarlos
> - Quilt puede importarlos o reindexarlos
> - la operación es **manual y explícita**
> - no hay autoingesta al abrir el Graph
> - no hay watch automático en v1
>
> Una vez ingeridos, la verdad canónica sigue siendo `quilt.db`."

`docs/graph-space-migration-plan.md` lines 192-202 (Phase 9) reduces that to a three-rule spec:

> "- scan manual
> - importar o reindexar manualmente
> - sin watch en v1"

So GS-9 must add an **opt-in, explicit, non-watching** surface to (a) discover compatible files in a graph directory, and (b) ingest or re-ingest them into the canonical `quilt.db`. The migration plan never enumerates what "compatible resources" means, which is a real ambiguity (see Knowledge Gaps).

### What already exists — three pieces are partially in place

Three pieces of import/migration infrastructure already exist in the codebase, but they were built for the legacy "vault" model and were never wired to the new Graph Space (ADR-0030):

#### 1. `MigrationEngine` — application-layer engine for Quilt-flavored Markdown

`crates/quilt-application/src/migration/migration_engine.rs:43-152` exposes a non-generic engine (`Arc<dyn PageRepository>` + `Arc<dyn BlockRepository>` + `Arc<dyn PropertyRepository>`, per ADR-0006 DDD shape, see `state.rs:78-89`):

```rust
pub struct MigrationEngine { /* 3 Arc<dyn Trait> */ }

impl MigrationEngine {
    pub async fn import_file(&self, source: &str, page_name: &str) -> Result<ImportResult, MigrationError>;
    pub async fn import_directory(&self, dir_path: &Path) -> Result<Vec<ImportResult>, MigrationError>;
}
```

Behavior:

- **Idempotent by page name** (`migration_engine.rs:70-76`): `get_by_name` first; if a page with that name already exists, the engine skips with a `warnings: vec!["Page 'X' already exists, skipping"]` and returns `pages_created: 0`. Re-running the import is a no-op for already-ingested files.
- **Idempotent by block content**: NONE. Two re-runs of a directory with the same files do not touch already-imported blocks. The page name is the only dedupe key. (Reindex on already-ingested files is currently a no-op — see "Reindex semantics" gap below.)
- **Format support**: ONLY Quilt-flavored Markdown. The parser (`md_import_parser.rs:45-218`) reads:
  - YAML frontmatter (with `---` markers, key:value pairs, comments/empty lines skipped)
  - Indent-based block tree (2 spaces = 1 level of nesting, `- ` bullet marker)
  - `key:: value` property lines (consumed by the engine in `migration_engine.rs:172-181` via `infer_property_value`)
- **Type inference** (`migration_engine.rs:215-247`): string → bool/int/float/date/datetime fallback chain, with NaiveDate detection for `YYYY-MM-DD` and ISO 8601 datetimes.
- **No recursive directory walking**: `import_directory` calls `std::fs::read_dir` (`migration_engine.rs:125`) — a single, non-recursive scan. Subdirectories are not descended into.
- **No file filter beyond extension**: only `.md` files are considered. Non-md files, hidden files, and the `.quilt/` dir itself are silently ignored. There is no allowlist/denylist beyond the extension.

#### 2. HTTP endpoint — `POST /api/v1/migration/md`

`crates/quilt-server/src/handlers/migration.rs:126-172` exposes one endpoint:

```rust
pub async fn migrate_md_import(
    Extension(page_repo): Extension<Arc<dyn PageRepository>>,
    Extension(block_repo): Extension<Arc<dyn BlockRepository>>,
    Extension(property_repo): Extension<Arc<dyn PropertyRepository>>,
    Json(body): Json<ImportMdRequest>,
) -> Result<(StatusCode, Json<ImportMdResponse>), AppError>
```

Routed at `POST /api/v1/migration/md` (`routes.rs:50`). Request body: `{ "path": String }` (camelCase). Returns totals across all files (pages_created, blocks_created, warnings[]) plus a per-file breakdown.

Security hardening already present (`migration.rs:67-108`):

- `validate_path` requires the path to be inside `QUILT_VAULT_BASE` (env var, default `cwd`)
- Canonicalizes the path to resolve symlinks
- Rejects symlinks (`fs::symlink_metadata` + `is_symlink()`)
- Enforces a hard cap of 10,000 files in the target directory (DOS guard)

**What the endpoint does NOT know**:

- The active graph root. The handler reads the path from the request body and validates it against `QUILT_VAULT_BASE`, but it never asks the `AppState` for the currently-open graph. The endpoint would happily import a directory that is not the active graph — that violates the §3 "Graph como unidad canónica" rule when invoked against the wrong directory.
- The §4 "manual y explícita" contract. There is no UI surface, no confirmation step, no preview/dry-run. The handler just runs.
- Whether the call is the first ingest or a reindex. The body shape is identical.

#### 3. Test infrastructure — comprehensive coverage of the engine

`crates/quilt-application/tests/migration_tests.rs` (571 lines, 40+ tests):

- Parser unit tests: frontmatter parsing, indent-based nesting, property line detection
- Type inference: int/float/bool/date/datetime/string fallbacks
- 11 fixtures in `crates/quilt-application/tests/fixtures/md_import/`: `simple.md`, `with_properties.md`, `nested_blocks.md`, `no_frontmatter.md`, `boolean_properties.md`, `numeric_properties.md`, `date_properties.md`, `complex_nested.md`, `multiline_content.md`, `empty_frontmatter.md`, `mixed_content.md`
- Engine integration: `import_file` happy path, idempotent re-import, `import_directory` with empty dir, `import_directory` with multiple files, properties propagation
- Property-based: parser does-not-panic proptest

This is a strong base. The parser + engine + tests are production-quality for what they do.

### What is missing (or partially built)

#### A. The endpoint is not bound to the active Graph

The handler uses `QUILT_VAULT_BASE` (an env var defaulting to `.`) — that is the *legacy* model. With ADR-0030 ratified, the canonical scope is the **active graph root** held in `AppState.last_opened_graph` (RwLock<Option<PathBuf>>, set by `handlers/graphs.rs:170-178` and `global_state_repo.set_last_opened_graph`). The migration endpoint must:

- Reject the call if no graph is open (503, mirroring `graph_space.rs` 503 behaviour)
- Default the target directory to the graph root (so the user is scanning "their own graph dir", not an arbitrary path)
- Still allow a subdirectory override (the user may have nested notes under `./projects/`)
- Validate the resolved path is `<= graph_path` (i.e. the user cannot escape the graph via `..`)

This is a real seam. Today the endpoint answers a different question ("can you import any directory?") than the one §4 actually asks ("can you ingest what's in the graph dir?").

#### B. No "scan only" or "reindex" semantics — only "import"

`MigrationEngine.import_file` is wired to `INSERT` pages and blocks. There is no:

- "Scan only" mode that returns the candidate file list (paths, sizes, modified_at) without mutating the database. This is the necessary pre-step of a "manual y explícita" UX — the user must see what will be ingested before approving.
- "Reindex" mode that updates an already-ingested page if the file changed on disk (e.g. `mtime > page.created_at`). The current idempotency-by-name is a no-op skip, not a reindex.
- "Diff" mode that returns the delta (new files, modified files, deleted files). Without diff, the user cannot reason about what will change.

The ADR-0030 §4 wording "importarlos o reindexarlos" is permissive about both modes, but the implementation today is import-only, and import-only is itself only idempotent on first call (it skips, not updates, on subsequent calls).

#### C. No Graph Space metadata in the result

`ImportResult` reports `pages_created` and `blocks_created` (per file and totals). It does NOT report:

- Which pages already existed (skipped) — only a single warning string per file
- How many blocks would have been reindexed (skipped blocks)
- The diff against current `quilt.db` state

For an "manual y explícita" UX, the user needs to see "found 14 files: 3 new, 11 already ingested, 2 modified" before clicking "Ingest". The handler cannot produce that today.

#### D. The `MigrationEngine` is non-recursive

`import_directory` does `std::fs::read_dir` once. A user with notes organized as `graph-root/notes/<topic>/<page>.md` will get only the top-level files ingested. A recursive walk is needed for any non-trivial directory layout. This is a real product gap and the simplest way to discover it is to try `just dev` and import from a real-world directory.

#### E. No CLI surface

`crates/quilt-platform/src/cli.rs` defines `quilt init/open/page/block/journal/query/search/list-pages/page-info`. There is no `quilt migrate-md <path>` subcommand. The CLI is graph-dir-aware (`resolved_graph_dir()` at `cli.rs:47-65`) and uses `AppServices` from `bootstrap`. Adding a `Migrate` subcommand is straightforward, but does not exist today.

#### F. No frontend surface

`quilt-ui/src/core/api-client.ts` (audited via grep) has no `importMarkdown` / `scanForImport` / `migrateMd` method. There is no UI button, modal, or command-registry command. The only UX surface is a manual `curl POST /api/v1/migration/md`, which violates "manual y explícita" from the user's perspective.

#### G. The `FileWatcher` exists but is correctly not wired

`crates/quilt-platform/src/watcher.rs:41-80` defines `FileWatcher` using the `notify` crate. It is **not** used in any production code path today. `domain_events.rs:9` defines `FileEventType` for the domain event bus. The event bridge (`event_bridge.rs:11, 137-150`) maps file events to "no-op" handlers in tests. This is consistent with ADR-0030 §4 "no watch automático en v1" — the wiring is intentionally absent. GS-9 must NOT activate the watcher.

#### H. The parser is single-format

`MigrationEngine` only knows Quilt-flavored Markdown. It cannot ingest:

- Plain Markdown (no `key::` syntax, no Quilt bullet structure) — would parse but lose the per-line `key::` semantics; the indent-based block tree still works for top-level bullets.
- Logseq-flavored `.md`/`.org` files — Logseq uses a different block convention (no `- ` bullet at top level, but indented `-` for children; property syntax is also `key::` so that part overlaps).
- Other text formats (`.txt`, `.org` plain) — not supported.

The migration plan never specifies a format list. This is a hard knowledge gap for proposal.

### What the architecture already supports

The good news is that the seams GS-9 needs are clean:

- **DDD layering holds.** The engine is in `quilt-application`, the handler in `quilt-server`, the entity in `quilt-domain`. Adding a new "scan" or "reindex" use case is a clean addition to the application layer, with a thin REST/MCP handler.
- **Graph root is discoverable.** `AppState.last_opened_graph: RwLock<Option<PathBuf>>` is the canonical source. Reading it from a handler is one `.read().await` away. The `graphs.rs:69-92` handler shows the pattern for "use the active graph, fail if none".
- **Graph validation is pure.** `quilt_platform::graph_validation::validate_graph_layout` is side-effect-free (`graph_validation.rs:127-188`) and can be called before any file scan to fail explicitly (no §6 silent re-create).
- **Idempotency-by-name is already proven.** The 8 engine tests in `migration_tests.rs:443-462` and the handler's `validate_path` mean the "manual, re-runnable" path is already working at the page-name level.
- **`InMemoryRepo` and `MockPropertyRepo` patterns** are in `quilt-test-helpers` (and inline in `migration_tests.rs:20-92`). New use cases (scan, reindex, diff) can be tested without SQLite.

---

## Context Quality

- **Level: C2** — The migration engine, the HTTP endpoint, the format support, the security validation, and the test fixtures are all on file and verified. The gaps are: (1) the endpoint is not bound to the active graph root, (2) no scan/reindex/diff semantics, (3) no recursive walk, (4) no CLI/UI surface, (5) no specification of what "compatible resources" means beyond `.md`.
- **Evidence present:**
  - ADR: `docs/adr/0030-graph-space-journal-first-lifecycle.md` §4 (verbatim quoted)
  - Plan: `docs/graph-space-migration-plan.md` Phase 9 (lines 192-202)
  - Roadmap: `docs/ROADMAP.md:279` (GS-9 🔲), Phase 10 status table
  - Engine: `crates/quilt-application/src/migration/migration_engine.rs` (153 LOC), `md_import_parser.rs` (259 LOC)
  - Handler: `crates/quilt-server/src/handlers/migration.rs` (177 LOC), routed at `routes.rs:50`
  - Tests: `crates/quilt-application/tests/migration_tests.rs` (571 LOC, 40+ tests)
  - Fixtures: `crates/quilt-application/tests/fixtures/md_import/` (11 files)
  - Graph bootstrap: `crates/quilt-platform/src/init.rs`, `graph_validation.rs`
  - Graph state: `crates/quilt-server/src/state.rs`, `handlers/graphs.rs:69-184`
  - Domain language: `CONTEXT.md` entries for "Grafo", "Graph Space", "Grafo pesado" (no dedicated entry for "importar" or "ingerir" — gap)
  - Reversa context: Logseq's `.md` block format (overlap with current parser)
  - Existing precedent: GS-7 explore (sdd/explore/graph-space-metadata-gs7/explore.md), GS-8 explore (sddk/contextual-right-sidebar-gs8/explore.md)
- **Missing context:**
  - The exact list of "compatible resources" — `.md` is what the engine supports, but the plan never says whether plain Markdown, Logseq files, `.org`, etc. are in scope for v1
  - Whether reindex should detect file changes (mtime/hash) and overwrite, or only ingest new files
  - The "manual y explícita" UX — what is the confirmation step? A scan/preview, a diff, a CLI prompt, a UI modal?
  - The relationship between ingestion and `GraphSpace.path` (GS-7 metadata) — should ingestion update `path` or only be allowed if the active graph is the target?
  - The recursion policy — subdirectory support, hidden file policy, symlink policy beyond the handler's symlink-reject
  - Whether the existing `MigrationError` enum (PageAlreadyExists, InvalidPath, Io) needs new variants for "scan" and "reindex" semantics
- **Recommended effort:** **deepen** in proposal phase. The two design choices that need to be made explicit before writing specs are: (a) what counts as a "compatible resource" in v1, and (b) what "reindex" means given the current skip-on-name-match behavior. Both are blockers for writing a clean spec.

---

## Knowledge Coverage

| Class | Status | Evidence | Gap Impact |
|------|--------|----------|------------|
| Roadmap/Backlog | present | `docs/ROADMAP.md:279` (GS-9 🔲), `docs/graph-space-migration-plan.md` Phase 9 lines 192-202, Phase 10 table line 265-286 | Clear target, no ambiguity on what GS-9 is. The plan is short — only 3 rules. |
| Work Items | stale | No SDDK change folder for GS-9 yet. `openspec/changes/` has no GS-9 entry. The prior `graph-space-migration-phases-1-4/` covers Phases 1-4. | Blocks parallel work; proposal phase must spin up `sddk/manual-resource-ingestion-gs9/` and `openspec/changes/manual-resource-ingestion-gs9/`. |
| Architecture/ADRs | present | ADR-0030 §4 is fully ratified; `docs/adr/0030-graph-space-journal-first-lifecycle.md` is the source of truth. No conflicting ADRs. | The §4 wording is short and clear. The ambiguity is in §4's *implications*, not its text. |
| Ownership | partial | `MigrationEngine` lives in `quilt-application/src/migration/`, owned by the application layer. The handler is in `quilt-server/src/handlers/`, owned by the server. The graph state is in `quilt-server/src/state.rs`, owned by the server. | The graph-root binding question is a server-side concern; the engine itself does not need to know the graph root. Clear ownership. |
| Learnings | present | 40+ tests in `migration_tests.rs` document the engine's current behavior. GS-7 explore (sdd/explore/graph-space-metadata-gs7/explore.md) and GS-8 explore (sddk/contextual-right-sidebar-gs8/explore.md) are the kernel cadence precedents. | Pattern to follow; the existing handler/engine structure is a clean starting point. |

---

## Problem Taxonomy

| Axis | Applies | Evidence |
|------|---------|----------|
| Domain modeling | **Yes** | New verbs: "scan" (discover), "reindex" (sync), "diff" (preview). New entity concepts: "ingestion candidate" (file path + mtime + size + format), "ingestion plan" (new + modified + skipped + deleted). The `ImportResult` struct is too coarse for an "manual y explícita" UX — needs to surface per-file decisions. |
| Boundary/seam | **Yes** | (a) The handler's path scope is `QUILT_VAULT_BASE` (legacy env), not the active graph. This is a hard seam to fix. (b) The engine does not know the graph root — it imports whatever path is given. (c) The `validate_path` logic in `migration.rs:67-108` is duplicated by `graphs.rs:69-92`; both can use `quilt_platform::graph_validation`. |
| Coupling/connascence | **Low-Medium** | The engine is decoupled (uses trait objects, not generics). The handler is decoupled (uses `Extension<Arc<dyn Trait>>`). Adding new endpoints follows the existing pattern. The risk is connascence of meaning: "ingest" and "reindex" both touch the engine but have different semantics, and confusing them produces subtle bugs. |
| API contract | **Yes** | Need at least three new endpoints to honor §4 properly: (1) `GET /api/v1/migration/candidates?path=...` (scan only, no writes), (2) `POST /api/v1/migration/md` (existing — refactor to default to graph root and accept plan), (3) `POST /api/v1/migration/reindex` (new — detect changes since last import). The current contract accepts a free-form `path` and the `QUILT_VAULT_BASE` env var, which is the legacy shape. |
| Refactor/legacy | **Yes** | The `validate_path` function in `migration.rs:67-108` is largely a re-implementation of graph validation. With the active graph now resolvable from `AppState`, the handler should use the graph root instead. This is a small refactor but it changes the contract: the handler now requires an open graph and a path *within* it. |
| Event/CQRS | **No** (v1) | §4 forbids watch. The existing `FileEventType` machinery is dormant. GS-9 needs no event subscription. The "scan" is a one-shot REST call, not a stream. |
| Testing | **Yes** | Existing tests cover the engine. New tests needed: (1) handler refuses path outside graph root, (2) handler refuses when no graph is open, (3) scan endpoint returns the expected file list, (4) reindex detects a modified file, (5) reindex is idempotent for unchanged files. The 11 existing fixtures can be reused. |
| Security/operations | **Yes** | (a) Path validation must use the active graph root as the base, not `QUILT_VAULT_BASE` (which is a different env). (b) Symlink policy is already in place. (c) File count cap (10k) is in place. (d) DOS guard via canonicalize is in place. (e) New concern: reindex must not let a malicious file overwrite canonical content without the user explicitly confirming the change. |

---

## Domain Language And Invariants

### Domain Language (resolved)

- **Recurso compatible** (compatible resource) — A file in the graph directory that the migration engine knows how to parse. In v1, this is concretely `.md` files in Quilt-flavored format. The plan does not enumerate the list, but the engine is the de-facto source of truth.
- **Ingerir** (ingest) — One-shot import of a file's content into `quilt.db` as pages and blocks. Idempotent by page name.
- **Reindexar** (reindex) — Update the canonical content of an already-ingested page/block from the source file. Currently a no-op (the engine skips). To be defined: by mtime? by content hash? by file path stored in `pages.file_id`?
- **Scan manual** (manual scan) — A read-only operation that lists candidate files in the graph directory. No writes.
- **Graph root** — The user-chosen directory; the canonical database lives at `<graph_root>/.quilt/quilt.db`. Resolved from `AppState.last_opened_graph` at handler time.
- **Migración** (migration) — Legacy term for "ingestion". Still used in module/endpoint names (`MigrationEngine`, `POST /api/v1/migration/md`). The kernel proposal may rename to "ingest" or keep "migration" for backwards compat with the F21 module.

### Domain Language (ambiguous — must be resolved in proposal)

- **"Recursos compatibles"** — Is `.md` the only format in v1? What about plain Markdown, Logseq `.org`, plain `.txt`? The ADR is silent; the engine is the implicit answer. Proposal must decide and document.
- **"Reindexar"** — When the user reindexes, do we (a) skip files that haven't changed, (b) update blocks for changed files, (c) delete pages for deleted files, (d) all of the above? The current engine only does (a). Proposal must define the full reindex contract.
- **"Manual y explícita"** — Is the explicit step (a) a UI confirm button, (b) a CLI `--yes` flag, (c) a "plan" object that the user inspects, (d) all of the above per surface? The plan does not specify.
- **"Detectarlos"** — Does detection mean an automatic listing surfaced in the UI, or only "we can scan when asked"? The literal reading of §4 is "Quilt puede detectarlos" — capability, not action. So the UI is not required to surface candidates unprompted; a "scan" command from the user is the manual trigger.

### Invariants (from ADR-0030 §4, must hold)

- No auto-ingestion when opening a graph
- No file watching in v1
- Operation is manual and explicit (user-initiated)
- After ingestion, `quilt.db` is the canonical truth (the file is read, the DB is written; the file is not modified by Quilt)
- The user must always be able to see what will be ingested before it happens (manual confirmation)
- The operation must be re-runnable (idempotency or explicit diff)

### Invariants (from ADR-0030 §6, indirectly binding)

- A Graph that fails validation must not be auto-repaired. If the graph root is invalid, GS-9 must fail explicitly (no fallback to "import anyway").

### Invariants (from existing implementation, must continue to hold)

- Pages are deduplicated by name (`migration_engine.rs:70-76`)
- The path is validated against the vault base (`migration.rs:67-108`) — but the BASE changes from `QUILT_VAULT_BASE` to the active graph root
- File count is capped at 10,000 (`migration.rs:97`)
- Symlinks are rejected (`migration.rs:87-91`)
- The format parser is the same — no new format without parser support

### Unknowns (explicit)

- Where does the scan live? CLI? UI? Both? The plan does not say.
- Does ingestion also write to `global.db` (e.g., for "last import" tracking)? Probably not — global state is for cross-graph facts, and ingestion is per-graph.
- Does the new "scan" endpoint require auth? Yes (Bearer), like every other `/api/v1/*` endpoint.
- Does the new "scan" endpoint need to live behind a feature flag? Possibly — if the engine path changes, the existing `/api/v1/migration/md` endpoint changes its scope from "any directory" to "inside the active graph". The breaking change is small but real.

---

## Knowledge Gaps

- **"Compatible resources" is undefined.** The plan says "recurso compatible" but does not enumerate. The engine only knows `.md` (Quilt format). Proposal must explicitly state: v1 = `.md` only. Future formats (Logseq, plain Markdown, `.org`) are not in scope for this change.
- **"Reindex" semantics are undefined.** The current engine skips-on-name. A real reindex needs a "change detection" key (mtime, content hash, or stored file path on the page entity). Proposal must choose one. `pages.file_id` exists in the schema (`page.rs:42`) but is unused today — it could be repurposed for a `file_path` column. This is a real design decision that touches the `Page` entity.
- **"Manual y explícita" UX is undefined.** Scan + preview + confirm is the natural shape, but the plan does not say. Proposal should propose a two-step flow: (1) `GET /api/v1/migration/candidates` returns the plan, (2) `POST /api/v1/migration/md` accepts the plan. The frontend and CLI both consume this. This is not a hard blocker, but the spec must be consistent across surfaces.
- **Recursion policy is undefined.** The current engine does not recurse. The plan does not say. A user with `graph-root/notes/<topic>/<page>.md` will silently lose pages. Proposal should default to **recursive with depth limit** (e.g., 8 levels), with an opt-out flag.
- **The handler's `QUILT_VAULT_BASE` is a legacy artifact.** It dates from the pre-Graph-Space model. With the active graph root now resolvable, the env var should be retired. Proposal should call this out as a breaking change to the handler's contract.
- **No MCP tool exists for ingestion.** The MCP server (`crates/quilt-mcp/src/server.rs:243`) does not expose a migration tool. Per ADR-0001 (MCP-first), if the operation is available to the user, it should also be available to agents. Proposal should add an MCP tool in parallel with the REST surface.
- **No STALE folder for the existing `/api/v1/migration/md` endpoint.** The endpoint is current and used (per the engine tests and the handler's `validate_path` security). GS-9 will refactor it, not replace it.
- **No test for the handler.** `tests/migration_tests.rs` covers the engine, not the handler. The handler's `validate_path` and `import_directory` are uncovered. Proposal should add a handler test using `tower::ServiceExt::oneshot` + `axum::body::Body`.

---

## Affected Areas

- `crates/quilt-application/src/migration/migration_engine.rs` — extend with `scan_directory()` and `reindex_directory()` methods; add `ImportPlan` / `IngestionCandidate` value objects. **Major change.**
- `crates/quilt-application/src/migration/md_import_parser.rs` — minor: add a "dry parse" mode that returns the AST without writing. **Minor change.**
- `crates/quilt-server/src/handlers/migration.rs` — replace `QUILT_VAULT_BASE` with the active graph root from `AppState`. Add `GET /candidates` endpoint. Add a `POST /reindex` endpoint. Refactor `validate_path` to delegate to `quilt_platform::graph_validation` (or factor out a shared helper). **Major change.**
- `crates/quilt-server/src/routes.rs` — add the new routes under `/api/v1/migration/`. **Minor change.**
- `crates/quilt-server/src/state.rs` — already exposes `last_opened_graph: Arc<RwLock<Option<PathBuf>>>`. No new fields needed. **No change.**
- `crates/quilt-application/src/use_cases/` — new use case: `MigrationUseCases` with `scan()`, `ingest(plan)`, `reindex()`. **New file.**
- `crates/quilt-domain/src/entities/page.rs` — `file_id` field exists but is unused. Decision needed: repurpose for `file_path` to support reindex, or leave as-is. **Decision needed.**
- `crates/quilt-domain/src/repositories/page_repository.rs` — add `get_by_file_path` if we go with file_path-based reindex. **Conditional change.**
- `crates/quilt-mcp/src/handlers/` — add `migrate_scan`, `migrate_ingest`, `migrate_reindex` tools. **New file.**
- `crates/quilt-platform/src/cli.rs` — add `migrate scan` and `migrate ingest <path>` subcommands. **Minor change.**
- `quilt-ui/src/core/api-client.ts` — add `scanForImport()`, `ingestMd()`, `reindexMd()` methods. **Minor change.**
- `quilt-ui/src/features/import/` — new feature folder: scan modal, plan preview, ingest/reindex buttons, result toast. **New module.**
- `quilt-ui/src/features/command-center/builtin-commands.ts` — add `migration/scan`, `migration/ingest`, `migration/reindex` commands. **Minor change.**
- `crates/quilt-application/tests/migration_tests.rs` — add tests for `scan_directory`, `reindex_directory`, the new use case. **Major addition.**
- `crates/quilt-server/tests/` — new handler integration tests. **New file.**
- `docs/adr/0030-graph-space-journal-first-lifecycle.md` — no change needed. §4 already binds.
- `docs/adr/` — possibly a new ADR for "in-place ingestion format scope" (`.md` only) and "reindex change detection key" (file_path vs. mtime). **Decision needed.**
- `docs/graph-space-migration-plan.md` — Phase 9 description could be expanded once v1 format scope is fixed. **Minor addition.**
- `CONTEXT.md` — add domain entries for "Ingerir", "Reindexar", "Recurso compatible", "Plan de ingestión". **Minor addition.**

---

## Options

| Option | Pros | Cons | Effort |
|--------|------|------|--------|
| **A. Bind existing endpoint to active graph; add scan endpoint; defer reindex.** Smallest change. Reuses the existing engine, the existing handler, the existing tests. The user gets scan + ingest; reindex is a follow-up. | Lowest risk. The endpoint is "active graph root by default" + scan-only endpoint. Breaking change to the contract (path is now relative to the graph root) is small. | Reindex is the harder half of §4 ("importarlos o reindexarlos"). Deferring it means a follow-up change. |
| **B. Scan + ingest + reindex, all in one change.** Honors §4 fully. | Cleanest semantics. One PR, one spec, one set of tests. | Larger PR (engine + 3 use cases + 3 REST endpoints + 3 MCP tools + 3 CLI subcommands + UI). Higher review cost. |
| **C. Reuse `MigrationEngine` as-is; only add the active-graph binding and a scan endpoint; treat reindex as a no-op skip (current behavior).** Minimum product change. | Almost no code change. | §4 says "reindex" explicitly. A no-op skip is not reindex. The change is incomplete. |
| **D. Replace the Markdown-only engine with a generic "resource" abstraction (format registry).** Future-proofs for `.org`, plain Markdown, etc. | Cleanly extensible. | New design surface (format registry, format adapters). Probably out of scope for GS-9; could be a follow-up "GS-9.5: format pluggability" change. |

**Persona recommendation:** **Option B with a v1 scope fence** — build scan + ingest + reindex together so §4 is fully honored, but pin v1 to Quilt-flavored `.md` only. Defer format pluggability (`.org`, plain Markdown) to a follow-up change. This matches the prior kernel pattern (e.g. GS-7 added metadata, GS-8 added the right sidebar, each scoped but complete).

---

## Entropy Envelope

- **Method:** heuristic (no CogniCode MCP available in this explore phase)
- **Coupling risk:** **medium**
  - The new `MigrationUseCases` will be a hub: REST handlers, MCP tools, CLI subcommands, and (later) UI commands all consume it. With the existing engine using `Arc<dyn Trait>` and the handler using `Extension<Arc<dyn Trait>>`, the DI shape is already proven. Risk is in the API surface, not the wiring.
  - The "reindex" change detection key is a connascence of value decision. If we store `file_path` on `Page` and re-read the file at reindex time, we couple `Page` to the filesystem layout. If we use mtime stored alongside the page, we couple the migration engine to filesystem metadata. Either is fine, but the choice must be explicit.
  - The `validate_path` logic is duplicated between `migration.rs` and `graphs.rs`. The new code should call a single shared helper (likely in `quilt_platform::graph_validation`).
- **OCP risk:** **low**
  - The handler is a thin shell over the engine. Adding new endpoints (scan, reindex) is additive. The engine's new `scan_directory` is additive. No existing behaviour is replaced.
- **Connascence of execution:** **low**
  - Scan and ingest are independent. Reindex depends on scan's output. The natural API is `scan → plan → ingest`. Each step is idempotent.
- **Connascence of meaning:** **medium**
  - "Ingest" vs. "reindex" is the trap. The current engine uses "ingest" semantics (skip on duplicate). The user-facing surface must use both terms correctly: "ingest" creates new content, "reindex" updates existing content. Misuse in the UI or CLI is a real risk.

---

## Recommendation

**Recommended path:** Option **B with v1 scope fence**.

- Add `scan_directory(path)` to `MigrationEngine` — returns `Vec<IngestionCandidate>` (path, mtime, size, format) without writing.
- Add `reindex_directory(path, plan)` to `MigrationEngine` — for each file in the plan, if a page with the matching name exists and the file mtime is newer than the page's `updated_at`, re-parse and update blocks; if no page exists, treat as ingest. For files whose mtime is older, skip (no-op).
- Add `MigrationUseCases` in `quilt-application/src/use_cases/migration.rs` — three methods: `scan(path)`, `ingest(path, approved_plan)`, `reindex(path)`. Use the existing `MigrationEngine` underneath.
- Refactor `quilt-server/src/handlers/migration.rs`:
  - Replace `QUILT_VAULT_BASE` with the active graph root from `AppState.last_opened_graph` (fail 503 if no graph open).
  - Add `GET /api/v1/migration/candidates?path=...` (uses `scan`).
  - Keep `POST /api/v1/migration/md` but change its contract: `path` is now *relative to the graph root*; the handler resolves and validates against the active graph.
  - Add `POST /api/v1/migration/reindex` (uses `reindex`).
- Add 3 MCP tools in `quilt-mcp`: `quilt_migration_scan`, `quilt_migration_ingest`, `quilt_migration_reindex` (per ADR-0001).
- Add 2 CLI subcommands: `quilt migrate scan`, `quilt migrate ingest [--plan <file>]`. `quilt migrate reindex` follows.
- Add a UI feature: `quilt-ui/src/features/import/` with a `MigrationPanel` accessible from the right sidebar (consistent with GS-8 placement) and from a new command `Cmd+Shift+I` ("Migration: scan and ingest").
- Recursive walk with depth limit (default 8, configurable via query param).
- v1 format scope: **Quilt-flavored `.md` only**. Logseq, plain Markdown, `.org` are out of scope and explicitly documented in the proposal.
- Page/file association: store the relative `file_path` in `pages.file_id` (repurpose the field; it's currently unused) so reindex can locate the page from a file path. This is a small, well-scoped schema change.

**Specific proposal-phase deliverables:**

1. Define the `IngestionCandidate` and `IngestionPlan` value objects (file path, mtime, size, decision: new/skip/reindex).
2. Define the `MigrationUseCases` interface.
3. Refactor the handler's path validation to use the active graph root.
4. Add the scan + reindex endpoints (REST + MCP + CLI).
5. Decide and document the reindex change detection key (proposal recommends `pages.file_id` + file mtime).
6. Define the recursion policy (depth limit, hidden file policy, symlink policy).
7. Add a UI surface (MigrationPanel + command) consistent with GS-8.
8. Add comprehensive tests: scan, ingest, reindex, handler authorization, path scope, recursion edge cases.
9. Document the v1 format scope (`.md` only) in `CONTEXT.md` and a new ADR-0032 "In-place ingestion format scope" (or amend the proposal to supersede §4 ambiguity).
10. Decide whether to retire `QUILT_VAULT_BASE` (recommend yes, with a deprecation window).

---

## Ready For Proposal

**Yes.** Context quality is C2. The migration engine and HTTP endpoint are real and verified; the gaps are well-defined. The two open questions that must be resolved in the proposal (not blocking but central) are: (1) v1 format scope (`.md` only is recommended), and (2) reindex change detection key (`pages.file_id` + mtime is recommended).

The kernel proposal cadence for prior GS-7 and GS-8 changes is the clear precedent: a new `sddk/manual-resource-ingestion-gs9/` change folder, a matching `openspec/changes/manual-resource-ingestion-gs9/` folder, and the standard init → explore → propose → spec → design → tasks → apply → verify → archive flow.

Recommended next phase: `sddk-propose` with a concrete proposal that locks down (a) v1 format scope, (b) reindex semantics, (c) the scan/ingest/reindex API shape, and (d) the UI/CLI/MCP surfaces.

---

## Return Envelope

**Status:** success
**Summary:** Explored GS-9 — Quilt has a working `MigrationEngine` and `POST /api/v1/migration/md` endpoint, plus 40+ tests and 11 fixtures, but the engine is graph-root-blind (uses legacy `QUILT_VAULT_BASE` env), the handler is not bound to the active graph, the format scope is implicitly `.md` only, recursion is missing, and there is no scan/reindex/diff surface. The plan's three rules (scan manual, importar o reindexar manualmente, sin watch) are achievable with three new use cases and three new endpoints, all reusing the existing engine.
**Artifacts:**
- File: `sddk/manual-resource-ingestion-gs9/explore.md` (this file)
- Engram: `sddk/manual-resource-ingestion-gs9/explore` (saved)
**Next:** sddk-propose (with v1 format scope, reindex semantics, and the active-graph-root binding as the three preconditions)
**Risks:**
- The §4 wording "recursos compatibles" is undefined. Proposal must lock the v1 scope to `.md` (Quilt format) to keep the change scoped. A future "GS-9.5: format pluggability" can extend to Logseq/plain Markdown/.org.
- The current handler is bound to `QUILT_VAULT_BASE` (legacy env), not the active graph root. This is a contract change for the existing endpoint. A deprecation window or a new endpoint path may be needed.
- "Reindex" semantics are undefined. The current engine is a skip-on-name-match no-op, not a reindex. A real reindex needs a change-detection key — `pages.file_id` is a natural choice (the field exists and is unused) but the schema migration must be planned.
- The migration engine is non-recursive. A user with nested notes (the common case) will lose pages. The proposal must add a recursive walk with a depth limit.
- There is no UI surface today. Adding one touches `api-client.ts`, the right sidebar (GS-8 territory — coordination needed), and the command palette. The proposal must scope the UI work to avoid stepping on GS-8.
- The `pages.file_id` field repurposing is a schema change. If GS-7 already shipped with this field, the migration must be coordinated; if not, this change should ship the migration.
**Skill Resolution:** paths-injected (sddk-explore skill loaded by orchestrator)
**Context Quality:** C2
