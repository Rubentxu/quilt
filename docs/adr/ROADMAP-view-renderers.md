# Quilt View Renderers — Implementation Roadmap

> Last updated: 2026-06-11
> Related ADRs: ADR-0014 Strategy Selector, ADR-0016 SavedView, ADR-0022 Template Cards, ADR-0023 Block Renderer Registry, ADR-0024 Property View Renderers

## Legend

| Status      | Meaning              |
|-------------|----------------------|
| Done        | Implemented and merged |
| In Progress | Currently being worked on |
| Planned     | Scoped, not started |
| Blocked     | Waiting on a dependency |
| Future      | Nice-to-have, post-MVP |

---

## Phase 1: Registry Migration (Foundation)

**Goal**: Migrate remaining inline renderers to `BlockRendererRegistry`. Convert placeholder views to real Gallery + List renderers.

| #    | Task                                                                                  | Status  | Files |
|------|---------------------------------------------------------------------------------------|---------|-------|
| P1.1 | Migrate AgentRun header to registry                                                   | Planned | `BlockRow.tsx` → `rendering/AgentRunRenderer.tsx` |
| P1.2 | Migrate Priority badge to registry                                                    | Planned | `rendering/PriorityRenderer.tsx` |
| P1.3 | Migrate Created-by badge to registry                                                  | Planned | `rendering/CreatedByRenderer.tsx` |
| P1.4 | Migrate Annotation badge to registry                                                  | Planned | `rendering/AnnotationRenderer.tsx` |
| P1.5 | Migrate `PropertyStrip` to registry                                                   | Planned | `rendering/PropertyStripRenderer.tsx` |
| P1.6 | Migrate `InlinePropertyBadges` to registry                                            | Planned | `rendering/InlinePropertyBadgesRenderer.tsx` |
| P1.7 | `HeadingRenderer` (h1/h2/h3 tags + font-size)                                         | Planned | `rendering/HeadingRenderer.tsx` |
| P1.8 | `QuoteRenderer` (left border + italic)                                                | Planned | `rendering/QuoteRenderer.tsx` |
| P1.9 | `CodeBlockRenderer` (monospace framing)                                               | Planned | `rendering/CodeBlockRenderer.tsx` |
| P1.10 | `DividerRenderer` (`<hr>` visual)                                                    | Planned | `rendering/DividerRenderer.tsx` |
| P1.11 | `BulletListRenderer` (bullet symbol)                                                  | Planned | `rendering/BulletListRenderer.tsx` |
| P1.12 | `NumberedListRenderer` (sequential counter)                                           | Done    | `rendering/NumberedListRenderer.tsx` |
| P1.13 | `ImageRenderer` (preview embed)                                                       | Done    | `rendering/ImageRenderer.tsx` |
| P1.14 | `GalleryViewRenderer` (card grid with cover)                                          | Planned | `rendering/GalleryViewRenderer.tsx` → replaces `SavedViewBlock` placeholder |
| P1.15 | `ListViewRenderer` (compact list with metadata)                                       | Planned | `rendering/ListViewRenderer.tsx` → replaces `SavedViewBlock` placeholder |
| P1.16 | `StrategyViewRenderer` (migrate `SavedViewBlock` dispatch to registry)                | Planned | `rendering/StrategyViewRenderer.tsx` |
| P1.17 | `BlockRow` clean — remove all inline render branches                                   | Planned | `BlockRow.tsx`: target < 800 lines |

---

## Phase 2: ViewConfig & Property Schema

**Goal**: View configuration becomes persistable data. Property types gain schemas.

| #    | Task                                                                                  | Status  | Files |
|------|---------------------------------------------------------------------------------------|---------|-------|
| P2.1 | `ViewConfig` type definition (layout, filter, sort, groupBy, visibility, cardConfig)  | Planned | `shared/types/viewConfig.ts` |
| P2.2 | Serialize `ViewConfig` as block property                                              | Planned | `SavedViewBlock` reads/writes `ViewConfig` |
| P2.3 | `PropertySchema` type (`@options` for select, `@dateFormat`, `@relationTarget`)        | Planned | `shared/types/propertySchema.ts` |
| P2.4 | Per-property cell renderer in `TableView` (select=pill, date=calendar, boolean=checkbox) | Planned | `TableView` cell renderers |
| P2.5 | Filter UI: typed operators per property (text contains, number `>`, select `=`, date before) | Planned | `QueryBuilder` typed filters |
| P2.6 | Multi-column sort UI in `TableView` headers                                           | Planned | `TableView` sort chips |
| P2.7 | "Add column" button discovers properties from query results                          | Planned | `TableView` column discovery |

---

## Phase 3: Database Block & Rich Properties

**Goal**: First-class `Database` block type. Enriched property types.

| #    | Task                                                                                  | Status  | Files |
|------|---------------------------------------------------------------------------------------|---------|-------|
| P3.1 | Multi-select property type (badge array in cells, multi-pill editor)                 | Planned | `types/api.ts` + `PropertyPanel` |
| P3.2 | Status property type (3-group: To-do / In Progress / Complete, colored)              | Planned | `types/api.ts` + cell renderers |
| P3.3 | Person property type (avatar + name, user picker)                                     | Planned | `types/api.ts` + cell renderers |
| P3.4 | Files / Media property type (thumbnail, upload, preview)                              | Planned | `types/api.ts` + cell renderers |
| P3.5 | URL property type (clickable link, favicon)                                           | Planned | `types/api.ts` + cell renderers |
| P3.6 | Relation property type (linked page pill, bidirectional)                              | Planned | `types/api.ts` + cell renderers |
| P3.7 | `CalendarViewRenderer` (month grid with events)                                       | Planned | `rendering/CalendarViewRenderer.tsx` |
| P3.8 | `TimelineViewRenderer` (horizontal bars by date range)                                | Planned | `rendering/TimelineViewRenderer.tsx` |
| P3.9 | Database block type with 1 `dataSource` + N views                                     | Planned | `types/api.ts` + `DatabaseBlock` |
| P3.10 | Multi-view on same data: switch layout tabs                                           | Planned | `DatabaseBlock` view tabs |

---

## Phase 4: Advanced Views & MCP

**Goal**: Chart view, Feed view, full MCP tooling for views.

| #    | Task                                                                                  | Status  | Files |
|------|---------------------------------------------------------------------------------------|---------|-------|
| P4.1 | `ChartViewRenderer` (bar, line, donut from aggregate data)                            | Future  | `rendering/ChartViewRenderer.tsx` |
| P4.2 | `FeedViewRenderer` (chronological stream)                                             | Future  | `rendering/FeedViewRenderer.tsx` |
| P4.3 | Cross-page data sources for views                                                     | Future  | `SavedViewBlock` cross-page queries |
| P4.4 | Inline database embedding (drag from sidebar → inline in page)                        | Future  | `DatabaseBlock` inline mode |
| P4.5 | MCP: `quilt_view_create` / `update` / `delete`                                        | Future  | `quilt-mcp` view handlers |
| P4.6 | MCP: `quilt_database_create_inline`, `promote_to_fullpage`                            | Future  | `quilt-mcp` database handlers |
| P4.7 | Conditional color rules per view                                                      | Future  | `ViewConfig.colorRules` |
| P4.8 | Formula / Rollup properties (computed columns)                                        | Future  | `types/api.ts` + formula engine |

---

## Phase 5: Nice-to-Have (Future)

| #    | Task                                       | Status  |
|------|--------------------------------------------|---------|
| P5.1 | MermaidRenderer (diagrams in code blocks)  | Future  |
| P5.2 | EmbedRenderer (YouTube, Figma, oEmbed)     | Future  |
| P5.3 | CalloutRenderer (tip/warning/danger)       | Future  |
| P5.4 | Gallery lightbox / image zoom              | Future  |
| P5.5 | Map view with geospatial pins              | Future  |
| P5.6 | Math block (LaTeX / KaTeX rendering)       | Future  |
| P5.7 | Code syntax highlighting per language      | Future  |

---

## Dependency Graph

```
Phase 1 (Registry Migration) ──── necessary for ────▶ Phase 2 (ViewConfig)
Phase 2 (ViewConfig)          ──── necessary for ────▶ Phase 3 (Database Block)
Phase 3 (Database Block)      ──── necessary for ────▶ Phase 4 (Advanced Views)

Phase 5 (Nice-to-Have)        ──── independent, can start any time
```

---

## Current BlockRow line count goal

| Milestone                              | BlockRow lines | Delta                                       |
|----------------------------------------|----------------|---------------------------------------------|
| Start (before registry)                | 1855           | —                                           |
| After TaskRenderer migration           | ~1887          | +32 (new imports + ctx + registry calls)    |
| After Phase 1 complete                 | < 800          | -1087 (render branches extracted to registry) |
