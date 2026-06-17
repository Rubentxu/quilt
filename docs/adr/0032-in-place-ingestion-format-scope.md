# ADR-0032: In-place ingestion format scope & reindex semantics

- **Date:** 2026-06-17
- **Status:** proposed
- **Binding contract:** ADR-0030 §4 (ratified)
- **Supersedes:** None
- **Superseded by:** None

## Context

ADR-0030 §4 defines the contract for manual resource ingestion from a graph
directory:

> "Si la carpeta del Graph contiene recursos compatibles:
> - Quilt puede detectarlos
> - Quilt puede importarlos o reindexarlos
> - la operación es **manual y explícita**
> - no hay autoingesta al abrir el Graph
> - no hay watch automático en v1
>
> Una vez ingeridos, la verdad canónica sigue siendo `quilt.db`."

The wording "recursos compatibles" and "reindexarlos" is intentionally broad
in ADR-0030 but must be narrowed to implementable scope. The existing
`MigrationEngine` (Quilt-flavored Markdown only) is the de-facto authority
but was never formally scoped.

## Decision

### 1. Format scope v1: Quilt-flavored `.md` only

In v1, "compatible resources" are files with `.md` extension that contain
Quilt-flavored Markdown (YAML frontmatter + indent-based block tree +
`key:: value` property syntax). Plain Markdown files with `.md` extension
are also ingestible (best-effort parsing). Non-`.md` files (`.org`, `.txt`,
`.csv`) are ignored.

**Rationale:**
- The existing `MigrationEngine` only knows this format.
- Format pluggability (format registry, Logseq `.org`) is deferred to a
  follow-up change ("format pluggability" — GS-9.5 or later).
- This avoids scope creep on GS-9 while fully honoring §4 for the dominant
  use case (users migrating from Logseq with `.md` files).

### 2. Reindex semantics: mtime-gated block replacement

"Reindexar" means: for an already-ingested file, if the file's modification
timestamp (`mtime`) on disk is newer than the stored `source_mtime` on the
Page entity, re-parse the file and replace its blocks in a single
transaction (delete-all-then-insert). If the mtime is unchanged, skip.
If the source file no longer exists on disk, skip — the canonical page
in `quilt.db` is never deleted by reindex.

**Rationale:**
- mtime is a cheap, universally available change detection signal (no
  content hashing required).
- Delete-all-then-insert is safe because blocks have no stable identity
  across parses (`Uuid::new_v4()` per block creation).
- Pages are never deleted — this preserves `quilt.db` as the canonical
  truth (ADR-0030 §4).

### 3. Manual y explícita: two-step scan → confirm

The "manual y explícita" contract is implemented as a two-step flow across
all surfaces (REST, MCP, CLI, UI):

1. **Scan** (read-only): returns an `IngestionPlan` with per-file status
   (`new` / `modified` / `skipped`) and a summary. No writes occur.
2. **Confirm**: the user explicitly invokes `ingest` (for `new` files) or
   `reindex` (for `modified` files), providing the approved plan.

The UI MigrationPanel does not auto-scan on open; the user must click
"Scan for files". The CLI requires an explicit confirm prompt (or `--yes`
flag). The MCP tools accept the plan as input.

### 4. Source tracking on Page

Each ingested Page records `source_path` (relative path to the `.md` file
within the graph root) and `source_mtime` (the file's mtime at ingestion
time). These are nullable columns on the `pages` table (migration 010).
Pages created through normal Quilt usage have `NULL` source tracking and
are never matched by ingestion or reindex.

### 5. Active graph root is the path authority

The `QUILT_VAULT_BASE` environment variable is retired. All migration
endpoints resolve paths against the active graph root held in
`AppState.last_opened_graph`. If no graph is open, endpoints return 503.
Paths that escape the graph root via `..`, symlinks, or absolute paths
return 400.

### 6. Concurrency: optimistic CAS

Concurrent reindex operations are safe via optimistic compare-and-swap on
`source_mtime`: the `UPDATE pages SET source_mtime = ?new WHERE id = ?id
AND source_mtime = ?old` query ensures only the first writer succeeds;
subsequent writers see the updated mtime and skip.

## Consequences

- GS-9 implementation is scoped and complete for `.md` users.
- Format pluggability is a separate, future concern (not blocking).
- `QUILT_VAULT_BASE` deprecation requires a one-release deprecation log
  window before removal (current: startup deprecation log only).
- `source_path`/`source_mtime` introduce a Meaning connascence pair (Page
  ↔ filesystem layout) that must be monitored in future changes.
