# ADR-0031: Right Sidebar Property Split — ADR-0020 Reconciliation

- **Status:** draft
- **Date:** 2026-06-17
- **Source:** GS-8 (graph-space-migration-plan Phase 8, ADR-0030 §14)
- **Supersedes:** ADR-0020 §"block-focus-populates-header" interaction

## Context

ADR-0020 (accepted, 2026-06-03) decided that:
1. Property editing lives in a **fixed header section** below the page title.
2. Block focus **populates the same header** — users never context-switch.
3. No right sidebar for properties in v1.

ADR-0030 §14 (accepted, 2026-06-17) explicitly requires a **contextual right sidebar** as the primary surface for:
- Rich property editing
- Metadata
- Contextual actions
- Semantic context

These two ADRs conflict on the property editing surface.

## Decision

ADR-0020 is **NARROWED**, not rescinded. The conflict is resolved as follows:

### What ADR-0020 still governs

- Page-level properties (title, tags, icon, etc.) remain in the **fixed header** below the page title.
- The 2-state CRDT invariant (empty/populated) for page-level properties is preserved.
- The header is the canonical surface for page-wide metadata.

### What ADR-0031 supersedes in ADR-0020

- ❌ "Block focus populates the same header" — REJECTED.
- Block-level property editing moves to the **right sidebar**.
- The header does NOT change when the user selects a different block.

### Resolution: Two-surface model

| Scope | Surface | Notes |
|-------|---------|-------|
| Page-level properties | Fixed header | Title, icon, tags, page-level custom props |
| Block-level properties | Right sidebar | All block-attached properties |
| Contextual actions | Right sidebar | Ranked main action per selection |
| Cognitive panels | Right sidebar | Agent activity, graph, insights |

### Why not merge into one surface?

1. **Category boundary**: Page properties are metadata about the document; block properties are metadata about content units. Separate surfaces enforce the distinction.

2. **CRDT safety**: Page header has 2 states (empty/populated). If block selection also populated the header, the 3-state problem resurfaces (empty/page-block/block).

3. **Cognitive load**: A header that changes on every block selection would be disorienting. A stable page header + contextual sidebar is the Logseq pattern (confirmed in `docs/reversa/`).

## Consequences

### Positive
- ADR-0030 §14 satisfied: right sidebar is the primary contextual surface
- ADR-0020 page-level authority preserved: header stays stable
- Clear category boundary: page vs block property editing surfaces
- Logseq-confirmed pattern: sidebar-based block property editing

### Negative
- Two surfaces for properties (header + sidebar) requires users to learn which is which
- Implementation requires SelectionContext to drive sidebar content

### Neutral
- Block focus does NOT change the header — users must look at the sidebar for block properties
- ESC key clears block selection and returns to page-level context in sidebar

## Open Questions

1. Should block selection also show a breadcrumb in the header?
2. What happens to page-level custom properties when a block is selected — do they remain visible in header?
3. Mobile: does the bottom sheet show page properties (header) or block properties (sidebar)?

## Related Decisions

- ADR-0020: Property Editing Surface — Fixed Header (NARROWED)
- ADR-0030 §14: Contextual right sidebar (accepted)
- GS-8: Contextual right sidebar implementation

## Validation

1. Unit test: selecting a block shows block properties in sidebar, not header
2. Unit test: page-level properties remain in header regardless of block selection
3. E2E: `t r` shortcut toggles right sidebar visibility
4. E2E: right sidebar sections render based on SelectionContext
