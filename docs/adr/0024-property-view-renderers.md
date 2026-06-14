# ADR-0024: Property-driven View Renderers — Notion-style database views from typed properties

Status: proposed

## Context

ADR-0016 established `SavedView` as a block role (`type:: view`) composing a reference to a `Query` block via `data-source::`. The dispatcher recognizes 7 view types but only 2 have real renderers (table, kanban). The other 5 are dashed-border placeholders (list, graph, cards, calendar, timeline).

Comprehensive market research (2026-06-11) surveyed Notion (9 views), AFFiNE, AnyType, Obsidian Dataview, Logseq, Airtable, and Coda. Key findings:

### Industry standard view types

| View            | Notion | AFFiNE   | AnyType     | Dataview   | Airtable   | Quilt |
|-----------------|--------|----------|-------------|------------|------------|-------|
| Table           | yes    | yes      | yes (Grid)  | yes        | yes (Grid) | yes   |
| Kanban / Board  | yes    | yes      | yes         | —          | yes        | yes   |
| Gallery / Cards | yes    | WIP      | —           | yes (DV)   | yes        | no    |
| List            | yes    | WIP      | yes         | —          | yes        | no    |
| Calendar        | yes    | Planned  | yes         | yes        | yes        | no    |
| Timeline/Gantt  | yes    | Planned  | —           | —          | yes (paid) | no    |
| Chart           | yes    | —        | —           | —          | —          | no    |
| Feed            | yes    | —        | —           | —          | —          | no    |
| Map             | yes    | Planned  | —           | —          | —          | no    |

### Property types required for view rendering

Notion's system has ~25 property types. Quilt currently has 6 wire types: `string`, `number`, `boolean`, `date`, `select`, `page_ref`. Critical gaps:

- No `multi_select` (needed for multi-value badges in tables).
- No `status` as distinct from `select` (needed for board column grouping with predefined groups: To-do / In Progress / Complete).
- No `person`/`user` (needed for assignee columns/avatars).
- No `files`/`media` (needed for gallery covers, image previews).
- No `url` (needed for clickable links).
- No `relation` beyond `page_ref` (needed for linked databases).
- No `formula` or `rollup` (needed for computed columns).
- `select` has no `options[]` (needed for colored pills and board columns).
- No schema persistence — properties are free-form key/value blobs per block.

### Common architecture pattern across all surveyed tools

```
Database Block
  ├── DataSource: query or block collection
  ├── Schema: PropertyType[] (typed, with options per type)
  └── Views[]: { layout, filter, sort, groupBy, propertyVisibility, cardConfig }
       ├── View "Board"   → Kanban renderer
       ├── View "Table"   → Table renderer
       └── View "Gallery" → Gallery renderer
```

## Decision

### Phase 1 — ViewRenderers migrate to the BlockRendererRegistry

The existing `TableView` and `KanbanBoard` become registered renderers. `Gallery` and `List` become new renderers. The `SavedViewBlock` dispatcher becomes a `strategy='view'` renderer in the registry.

### Phase 2 — ViewConfig as persistable data

A `ViewConfig` object serialized on the view block carrying: `layout`, `filter`, `sort`, `groupBy`, `propertyVisibility`, `cardConfig`. This replaces hardcoded `DEFAULT_TABLE_COLUMNS` and implicit dispatch.

### Phase 3 — PropertySchema with enriched types

Properties gain first-class schemas: `Select { options[] }`, `MultiSelect { options[] }`, `Status { groups[] }`, `Person`, `Files`, `URL`, `Relation`, `Formula`, `Rollup`. Each type has a dedicated cell renderer in `TableView` and a card renderer in `Gallery`/`Kanban`.

### Phase 4 — Multi-view database block

A `Database` block type that holds 1 `DataSource` and N `Views`. Same data, multiple projections. Enable inline database embedding (drag-to-sidebar → promote-to-full-page).

### Design constraints recognized

1. Views are NOT entities — they are properties on a view block (ADR-0016 already decided this).
2. `SavedView` is a BLOCK ROLE (`type:: view`), not a separate entity.
3. `Query` blocks (`type:: query`) provide the data; `View` blocks (`type:: view`) project it.
4. Multiple views can reference the same query — same data, different renderers.
5. `CardRenderer` (from ADR-0022) wraps `BlockRow` from OUTSIDE; `ViewRenderers` live INSIDE `BlockRow` via the registry — they are different layers.

## Considered Options

1. **Create a separate "Database" entity** — rejected (ADR-0016). The role system is Quilt's type system. A database IS a block with `type:: view` composing a query.
2. **Merge Query and View into one block** — rejected (auto-grill Q004-P1). Property-bag anti-pattern. Composition over inheritance.
3. **Implement all 9 view types at once** — rejected. Incremental delivery. Phase 1: Table + Kanban + Gallery + List. Phase 2: Calendar + Timeline. Phase 3: Chart + Feed.
4. **Adopt Notion's full property type system immediately** — rejected. Incremental. Phase 1 uses existing 6 types. Phase 3 enriches.

## Consequences

- 5 placeholder view types become real renderers (starting with Gallery + List).
- `ViewConfig` becomes queryable/versionable as block properties (not implicit in code).
- Property type enrichment unlocks multi-select, status groups, person avatars, file previews.
- MCP tools gain: `quilt_view_create`, `quilt_view_update`, `quilt_view_delete`.
- Query DSL gains operators (`Contains`, `Between`, `GreaterThan`, `LessThan`) currently unreachable from the UI.
- The existing `SearchModal` "Save as View" preserves structured AST instead of flattening to text.

## References

- ADR-0014 — Strategy Selector (role dispatch lives in the registry).
- ADR-0016 — SavedView block role (foundation: view is a block with `data-source::`).
- ADR-0022 — Template-driven block cards (separate outer layer, not affected).
- ADR-0023 — Block Renderer Registry (Phase 1 mechanism for view renderers).
- auto-grill Q004-P1 / Q011-P2 (2026-06-07) — rejected property-bag anti-pattern.
