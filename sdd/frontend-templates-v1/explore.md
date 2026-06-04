# Exploration: frontend-templates-v1 (Wave 2, Stream E)

**Date**: 2026-06-04 | **Method**: CogniCode + manual code reading | **Strict TDD**: YES

## Executive Summary

Wave 2 Stream E is **feasible with moderate complexity**. The existing infrastructure is strong: `PropertyOp` (8 operators), `QueryAst` with `Table` variant, `TemplateUseCases` trait, and WASM bridge exports for `query_parse`/`query_validate` are all complete from Wave 1. The main work is frontend React components (F13, F17) and wiring the client-side AST through existing MCP tools (F18). G6 (Schema packs) requires a new metadata design — the current template metadata is limited to `card-shape::`, `icon::`, `cssclass::`. F15 (Reapply) needs a new application use case. F20 adds MCP tool wrappers. Total estimated LOC: ~1,800 (700 TSX, 600 Rust, 500 tests).

## Current State

**Template infrastructure**: `TemplateUseCases` trait with `list_templates()` and `get_template_schema()`. Implemented by `TemplateUseCasesImpl<PR, BR>` in `quilt-application`. Exposed via HTTP (`GET /api/v1/templates`, `GET /api/v1/templates/:name/schema`) and MCP (`quilt_list_templates`, `quilt_get_template_schema`). Template metadata reads `card-shape::`, `icon::`, `cssclass::` from the first block's properties. No schema pack concept exists yet.

**Property system**: `PropertyEntry` trait hierarchy (HasValue, HasTimestamp, Mergeable) complete. `DefaultPropertyEntry<V>` for Page.properties. `PropertyValue` enum (7 variants). `PropertyOp` enum with 8 operators (Equals, NotEquals, Contains, GreaterThan, LessThan, GTE, LTE, Between). `QueryAst` with `Property` variant using `PropertyOp`. `PropertyKeyResolver` for case-insensitive key lookup.

**Query DSL**: Full parser (`pest`-based) in `quilt-query`. `QueryAst` enum with 17 variants including `Table(Vec<QueryAst>)` and `Property { key, op, value, value2 }`. `QueryService` in `quilt-application` for server-side execution. WASM bridge exports `query_parse` and `query_validate` to JavaScript. MCP `quilt_query` tool returns plan only (AST + SQL, not block results). No client-side query execution path exists.

**Frontend**: React 19 + Vite 6 + TypeScript 5.7. `CardRenderer` (3 shapes + 2 placeholders). `BlockPropertiesPanel` (read/write individual properties). `SearchModal` (unified page+block search). `api-client.ts` with `listTemplates()`, `getTemplateSchema()`, `searchBlocks()`. No FilterChip, TableView, or Query execution components exist. `react-virtuoso` is available for virtualization.

## Per-Feature Analysis

### F13 — Filter-chips UI

**Feasibility**: HIGH. Low complexity.

**Existing code**: `BlockPropertiesPanel.tsx` renders properties as editable key-value rows with type icons. SearchModal debounces text input into FTS queries. No reusable chip component exists.

**What's needed**:
- `FilterChip` component: pill-shaped button with property key label, operator icon, value display, remove button
- `FilterChipGroup` component: horizontal wrapping row of chips with "+ Add filter" button
- Integration into `BlockPropertiesPanel` and/or a new query bar
- TypeScript types: `FilterChip { id, key, op: PropertyOp, value, value2? }`

**Dependencies**: F7 (property header), F11 (PropertyOp). **LOC estimate**: ~500 total.

---

### F17 — Table view UI

**Feasibility**: HIGH. `react-virtuoso` available. `QueryAst::Table` variant exists.

**Existing code**: `QueryAst::Table(Vec<QueryAst>)` defines structured columns from inner expressions. No table rendering exists. `react-virtuoso` package is already in `package.json` dependencies.

**What's needed**:
- `TableView` component: renders query results as columns using `react-virtuoso` for 1000+ row virtualization
- Column header with sort direction toggle (SortBy support)
- Row rendering with cell content (content, property values, markers)
- Integration with `PageView` or new route for showing query results
- TypeScript types: `TableQuery { columns: QueryAst[], sort?: SortBy, limit: number }`

**Dependencies**: F11 (QueryAst), F18 (QueryAst execution). **LOC estimate**: ~600 total.

---

### F18 — QueryAst execution

**Feasibility**: MEDIUM.

**Existing code**: WASM bridge exports `query_parse`/`query_validate`. MCP `quilt_query` returns plan only. `QueryService::execute()` runs server-side queries and returns blocks.

**Approach A — Proxy via new REST endpoint** (Recommended):
- Build `QueryAst` JSON client-side from filter chips
- Serialize as JSON, call `POST /api/v1/query`
- Server executes via `QueryService::execute()` and returns blocks
- **Pros**: Uses existing server-side execution infrastructure, type-safe, no SQL injection risk
- **Cons**: Requires new endpoint + network round-trip
- **Effort**: Medium

**Approach B — Direct client-side SQL**: REJECTED. Duplicates QueryCompiler in TypeScript, SQL injection risk.

**What's needed (Approach A)**:
- TypeScript `QueryAst` type (mirror of Rust enum via WASM JSON)
- `buildQueryAst(filterChips)` — construct AST from chip state
- `executeQuery(ast)` — serialize AST to JSON, call `POST /api/v1/query`
- New server endpoint: `POST /api/v1/query` accepting JSON AST

**Dependencies**: F11 (QueryCompiler), F2 (AggregateFn), F3 (PropertyOp). **LOC estimate**: ~500 total.

---

### G6 — Schema packs

**Feasibility**: MEDIUM.

**Existing code**: Template metadata is `card-shape::`, `icon::`, `cssclass::`. No schema pack concept exists.

**Approach B — String property + JSON parse** (Recommended):
- Schema pack JSON stored as `schema-pack::` string property on template page
- Fields: `card_shape`, `icon`, `cssclass` (existing), `link_verbs`, `default_properties`, `display_hints`
- Parse on read. No PropertyValue changes needed.
- New endpoint: `GET /api/v1/templates/:name/schema-pack`
- MCP tool: `quilt_get_template_schema_pack`

**Dependencies**: F5 (PropertyEntry), F19. **LOC estimate**: ~300 total.

---

### F15 — Reapply template

**Feasibility**: MEDIUM.

**Existing code**: Templates applied at page creation only. No reapply logic.

**Approach B — Timestamp comparison + simple override** (Recommended):
- `reapply_template(template_name, block_id)` use case
- Store `applied_at` timestamp on block when template is applied
- Reapply compares template properties vs current properties
- Simple override for V1: template properties always win (document tradeoff)
- REST endpoint: `POST /api/v1/templates/:name/reapply/:blockId`
- MCP tool: `quilt_reapply_template`

**Dependencies**: F5 (PropertyEntry), F4 (PropertyKeyResolver). **LOC estimate**: ~400 total.

---

### F20 — MCP template tools

**Feasibility**: HIGH.

**Existing code**: `TemplateToolHandler` with 2 tools. Follows `ToolHandler` trait pattern.

**What's needed**: Two new tools wrapping F15 + G6 use cases. **LOC estimate**: ~150 total.

---

## Connascence Map (Entropy Analysis — Protocol A)

**Method**: CogniCode + Heuristic | **Confidence**: estimated

| Component A | Component B | Connascence Type | I(bits) | Severity |
|-------------|-------------|------------------|---------|----------|
| FilterChip | PropertyOp | Name | 3.0 | ⚠️ Medium |
| FilterChip | BlockPropertiesPanel | Meaning | 1.5 | ⚠️ Low |
| TableView | QueryAst::Table | Type | 2.0 | ⚠️ Medium |
| QueryAst(client) | QueryAst(server) | Algorithm | 4.1 | ❌ High |
| SchemaPack | PropertyEntry | Type | 1.0 | ⚠️ Low |
| ReapplyTemplate | TemplateSchema | Meaning | 2.5 | ⚠️ Medium |
| MCP template tools | TemplateUseCases | Execution | 3.0 | ⚠️ Medium |

**Critical Pair (I = 4.1 bits)**: `QueryAst(client) ↔ QueryAst(server)`. Mitigation: use WASM `query_parse` JSON output as canonical AST source (no manual TypeScript mirroring).

**Coupling Score**: H_external ≈ 2.1 bits (moderate). Most features are pure extensions.

## Risks and Mitigation

| Risk | Severity | Mitigation |
|------|----------|------------|
| F18: Client/Server AST drift | HIGH | Use WASM `query_parse` JSON as canonical. Generate TS types from Rust serde. |
| G6: No consensus on schema pack format | MEDIUM | Define format before implementation. String property + JSON parse. |
| F15: Conflict resolution ambiguity | MEDIUM | Start with "override all" mode. Source tracking in V2. |
| WASM bridge `query_parse` unused from frontend | LOW | Wire to FilterChip → AST construction path. |

## Recommended Scope

**INCLUDE** (6 features):
1. F13 — Filter-chips UI
2. F17 — Table view UI
3. F18 — QueryAst execution (proxy via new REST endpoint)
4. G6 — Schema packs (string property + JSON parse)
5. F15 — Reapply template (timestamp comparison, simple override)
6. F20 — MCP template tools (thin wrappers)

**EXCLUDE / DEFER**: kanban-card/timeline-card shapes, PropertyValue extension, source tracking for reapply

**Recommended order**: F18 → F13 + F17 (parallel) → G6 → F15 → F20

## Ready for Proposal

**Yes** — all 6 features have clear approaches. The only design decision needed before spec: confirm G6 schema pack format (JSON-in-property).
