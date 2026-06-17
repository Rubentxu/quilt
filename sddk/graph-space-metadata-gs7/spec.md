# Kernel Specs: GS-7 — Graph Space Metadata

> Source: `sddk/graph-space-metadata-gs7/proposal` (Engram #2271)
> Date: 2026-06-17

## Router Context Used
- **Knowledge Coverage:** sufficient
- **Context Quality:** C3
- **Taxonomy:** persistence/schema, interface/crud, validation
- **Domain Language (resolved):** Graph Space, Graph Content, identity field, library key, singleton marker, canonical path
- **Domain Language (ambiguous):** None — icon representation resolved to `lib:key` format in exploration
- **Recommended Effort:** verify

## Knowledge Provenance
- **Scope source:** `sddk/graph-space-metadata-gs7/proposal` (Engram #2271)
- **Invariant source:** `docs/adr/0030-graph-space-journal-first-lifecycle.md` §§5-7,16-17; `docs/graph-space-migration-plan.md` Phase 7
- **Memory-only hints excluded from spec truth:** None — all invariants are durably recorded in ADR-0030

---

## Capability: graph-space-identity

### Requirement: Singleton graph metadata entity
The system SHALL maintain exactly one row of graph-space metadata per `quilt.db`, with identity fields `id`, `name`, `icon`, `description`, `color`, `path`, `created_at`, and `updated_at`.

The entity SHALL implement a `validate()` method that enforces:
- `name` is non-empty
- `icon` matches the library-key format `{lib}:{name}` where `{lib}` is a known icon library identifier and `{name}` is a non-empty slug
- `color` is a valid hex color string (`#RRGGBB` or `#RRGGBBAA`)
- `path` passes `validate_graph_layout` without error
- `created_at` is a non-zero UNIX epoch millisecond timestamp

#### Scenario: Default entity after bootstrap
**Given** a freshly initialized `quilt.db` with no prior `graph_space` row
**When** the repository reads the graph-space metadata
**Then** it returns a `GraphSpace` entity with sensible defaults: `name` derived from the directory basename, `icon = ""`, `description = ""`, `color = ""`, `path` set to the canonical graph root, `created_at > 0`, and `validate()` returns `Ok(())`

#### Scenario: Valid entity with library-key icon
**Given** a `GraphSpace` with `icon = "lucide:book-open"` and `name = "Research"` and `color = "#3b82f6"` and a valid `path`
**When** `validate()` is called
**Then** the result is `Ok(())`

#### Scenario: Reject empty name
**Given** a `GraphSpace` with `name = ""`
**When** `validate()` is called
**Then** the result is `Err(DomainError::InvalidConfiguration("name must be non-empty"))`

#### Scenario: Reject invalid icon format
**Given** a `GraphSpace` with `icon = "star"` (no library prefix)
**When** `validate()` is called
**Then** the result is `Err(DomainError::InvalidConfiguration("icon must be in lib:name format"))`

#### Scenario: Reject unknown icon library
**Given** a `GraphSpace` with `icon = "unknown-lib:star"`
**When** `validate()` is called
**Then** the result is `Err(DomainError::InvalidConfiguration("unknown icon library: unknown-lib"))`

#### Scenario: Reject invalid hex color
**Given** a `GraphSpace` with `color = "blue"`
**When** `validate()` is called
**Then** the result is `Err(DomainError::InvalidConfiguration("color must be a hex string (#RRGGBB or #RRGGBBAA)"))`

#### Scenario: Reject path that fails graph layout validation
**Given** a `GraphSpace` with `path` pointing to a non-existent directory
**When** `validate()` is called
**Then** the result is `Err(DomainError::InvalidConfiguration)` with a message describing the path validation failure

---

## Capability: graph-space-persistence

### Requirement: Singleton bootstrap on first read
The system SHALL bootstrap a single `graph_space` row with `id = 1` on the first read if no row exists, using `INSERT OR IGNORE` semantics. The bootstrap row SHALL set `created_at` to the current UNIX epoch milliseconds and `name` to the directory basename of the canonical graph root.

#### Scenario: Bootstrap on empty graph
**Given** a `quilt.db` with a `graph_space` table but zero rows, and a graph root directory `/home/user/my-graph`
**When** `get_graph_space()` is called
**Then** it returns a `GraphSpace` with `id = 1`, `name = "my-graph"`, `created_at > 0`, `path = "/home/user/my-graph"`, and `validate()` passes
**And** subsequent reads return the same row unchanged

#### Scenario: No bootstrap when row already exists
**Given** a `quilt.db` with an existing `graph_space` row where `name = "Research"` and `created_at = 1715900000000`
**When** `get_graph_space()` is called
**Then** it returns the existing row with `name = "Research"` and `created_at = 1715900000000`
**And** no new row is inserted

### Requirement: Singleton invariant enforcement
The system SHALL enforce that only row `id = 1` ever exists in the `graph_space` table via a SQL `CHECK (id = 1)` constraint at the database level.

#### Verification note
- The `graph_space` table DDL includes `CHECK (id = 1)`. Any `INSERT` with `id != 1` fails at the SQL layer.
- The repository's `INSERT OR IGNORE` bootstrap always uses `id = 1`, keeping the invariant.

---

## Capability: graph-space-query

### Requirement: Read graph metadata via REST
The system SHALL expose `GET /api/v1/graph-space` returning the current `GraphSpace` entity as JSON. The endpoint SHALL require Bearer auth.

#### Scenario: Read existing metadata
**Given** a `GraphSpace` persisted with `name = "Research"`, `icon = "lucide:flask-conical"`, `description = "Science notes"`, `color = "#8b5cf6"`, `created_at = 1715900000000`, `updated_at = 1715900100000`
**When** `GET /api/v1/graph-space` is called with a valid Bearer token
**Then** the response is `200 OK` with JSON:
```json
{
  "id": 1,
  "name": "Research",
  "icon": "lucide:flask-conical",
  "description": "Science notes",
  "color": "#8b5cf6",
  "path": "/home/user/graphs/research",
  "created_at": 1715900000000,
  "updated_at": 1715900100000
}
```

#### Scenario: Read with no graph open returns 503
**Given** no active graph (the server is in selector state)
**When** `GET /api/v1/graph-space` is called
**Then** the response is `503 Service Unavailable` with a body indicating that no graph-space is available

#### Scenario: Unauthorized read returns 401
**Given** no Bearer token
**When** `GET /api/v1/graph-space` is called
**Then** the response is `401 Unauthorized`

---

## Capability: graph-space-update

### Requirement: Partial update of graph metadata via REST
The system SHALL expose `PUT /api/v1/graph-space` accepting a JSON body with optional fields `name`, `icon`, `description`, `color`. Omitted fields SHALL retain their current values. The endpoint SHALL validate the merged result before persistence and SHALL require Bearer auth.

#### Scenario: Update name and icon
**Given** a `GraphSpace` with `name = "Old Name"`, `icon = "lucide:book"`, `created_at = 1715900000000`
**When** `PUT /api/v1/graph-space` is called with `{"name": "New Name", "icon": "lucide:flask-conical"}`
**Then** the response is `200 OK` with the updated entity
**And** `name = "New Name"`, `icon = "lucide:flask-conical"`, `created_at = 1715900000000` (unchanged), `updated_at` reflects the new timestamp

#### Scenario: Partial update — only description changes
**Given** a `GraphSpace` with `name = "Research"`, `icon = "lucide:book"`, `description = ""`
**When** `PUT /api/v1/graph-space` is called with `{"description": "New description"}`
**Then** `name` and `icon` remain unchanged, `description` becomes `"New description"`

#### Scenario: Reject empty name on update
**Given** an existing `GraphSpace` with `name = "Research"`
**When** `PUT /api/v1/graph-space` is called with `{"name": ""}`
**Then** the response is `422 Unprocessable Entity` with an error indicating `name must be non-empty`

#### Scenario: Reject invalid icon on update
**Given** an existing `GraphSpace` with `icon = "lucide:book"`
**When** `PUT /api/v1/graph-space` is called with `{"icon": "star"}`
**Then** the response is `422 Unprocessable Entity` with an error indicating invalid icon format

#### Scenario: Reject invalid color on update
**Given** an existing `GraphSpace` with `color = "#3b82f6"`
**When** `PUT /api/v1/graph-space` is called with `{"color": "not-a-color"}`
**Then** the response is `422 Unprocessable Entity` with an error indicating invalid hex color

#### Scenario: Reject invalid path on update
**Given** an existing `GraphSpace` with a valid `path`
**When** `PUT /api/v1/graph-space` is called with `{"path": "/nonexistent/graph"}`
**Then** the response is `422 Unprocessable Entity` with an error indicating the path does not exist

#### Scenario: Reject attempt to modify created_at
**Given** an existing `GraphSpace` with `created_at = 1715900000000`
**When** `PUT /api/v1/graph-space` is called with `{"created_at": 9999999999999}`
**Then** the `created_at` field in the body is silently ignored (or the request is rejected with `422` if the implementation chooses to reject immutable fields in the body)
**And** the persisted `created_at` remains `1715900000000`

### Requirement: Immutability of created_at
The system SHALL set `created_at` once at bootstrap and SHALL NOT overwrite it on any update.

#### Scenario: created_at unchanged across multiple updates
**Given** a `GraphSpace` bootstrapped with `created_at = 1715900000000`
**When** three successive `PUT` updates are made to `name`
**Then** `created_at` remains `1715900000000` after all three updates

---

## Capability: graph-space-path-detection

### Requirement: Path mismatch detection on read
The system SHALL compare the persisted `path` to the canonical graph root from the active connection. If they differ, the system SHALL return a typed error indicating the graph has been moved.

#### Scenario: Graph moved — path mismatch
**Given** a `GraphSpace` with `path = "/home/user/graphs/old-location"` persisted in `quilt.db`
**But** the database was opened from `/home/user/graphs/new-location`
**When** `get_graph_space()` is called
**Then** the repository returns `Err(DomainError::InvalidData("graph path mismatch: expected /home/user/graphs/old-location, found /home/user/graphs/new-location"))`
**And** the REST endpoint returns `409 Conflict` with a `GraphMoved` error code

#### Scenario: Path matches — no error
**Given** a `GraphSpace` with `path = "/home/user/graphs/research"` and the database is opened from `/home/user/graphs/research`
**When** `get_graph_space()` is called
**Then** the result is `Ok(GraphSpace { path: "/home/user/graphs/research", ... })`

---

## Capability: graph-space-mcp-resource

### Requirement: MCP resource for graph metadata
The system SHALL expose the graph-space metadata as an MCP resource `graph://metadata` with `get_graph_space` and `update_graph_space` tools, following the same semantics as the REST endpoints.

#### Scenario: MCP read graph metadata
**Given** a `GraphSpace` persisted with `name = "Research"`
**When** an MCP client calls the `get_graph_space` tool
**Then** it returns the same `GraphSpace` entity as `GET /api/v1/graph-space`

#### Scenario: MCP update graph metadata
**Given** a `GraphSpace` with `name = "Old"`
**When** an MCP client calls `update_graph_space` with `{"name": "New"}`
**Then** `name` is updated to `"New"` and the response matches the REST `PUT` response

#### Scenario: MCP update rejected with validation error
**Given** an MCP client calls `update_graph_space` with `{"color": "invalid"}`
**When** the tool executes
**Then** it returns an error response with the same validation detail as the REST `422` response

---

## Invariants Covered

| Invariant | Coverage |
|-----------|----------|
| Metadata lives inside `quilt.db`, not `global.db` | `graph-space-persistence` — all persistence scenarios assume `quilt.db` context |
| Singleton row: `CHECK (id = 1)` | `graph-space-persistence: Singleton invariant enforcement` — verification note |
| `created_at` immutable after bootstrap | `graph-space-update: Immutability of created_at` — scenario + explicit requirement |
| `path` must pass `validate_graph_layout` | `graph-space-identity: Reject path that fails graph layout validation` |
| Identity fields editable from graph settings, not selector | Documented in ADR-0030 §17 but UI surface is OUT OF SCOPE for this change; spec covers API/MCP only |
| Moved graph must fail explicitly (ADR-0030 §6) | `graph-space-path-detection: Graph moved — path mismatch` |

## Open Questions

- **Icon library allowlist**: The exact set of allowed library keys (e.g., `lucide`, `tabler`) is an app-layer decision. The spec constrains format (`lib:name`) but the allowlist enumeration belongs in the design phase or a shared constant module.
- **Color format**: `#RRGGBBAA` support is listed but not confirmed by any ADR — design phase should decide whether to accept 8-digit hex or stay with 6-digit only.
- **Moved graph recovery UX**: This spec defines detection; the recovery flow (alert the user, offer to rebind, show selector) is a separate frontend task outside this change scope.
- **created_at rejection strategy on PUT**: The spec allows two approaches for handling `created_at` in the update body — silent ignore or explicit rejection. The design phase should pick one for consistency with the existing `user_settings` behavior.
