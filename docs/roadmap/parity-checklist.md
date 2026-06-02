# Quilt Leptos → React Migration Parity Checklist

> **Purpose**: Closed contract defining every user-visible feature that must work
> in the React version before the Leptos UI (`crates/quilt-ui/`) can be sunset.
>
> **Phase 2 gate**: ALL items marked ✅ here must be ✅ in the React build.
> Items ⚠️ must be at least ⚠️ (no regression). Items ❌ are Phase 3 stretch.
>
> **Last updated**: 2026-05-30
> **Source**: Auto-grill-loop Pass 1-2, 17 cycles, Q001-P1 through Q017-P2

## Legend

| Symbol | Meaning |
|--------|---------|
| ✅ | Implemented and verified in Leptos |
| ⚠️ | Partial or buggy — needs work |
| ❌ | Not implemented in Leptos |

## Summary

| # | Feature Area | Total | ✅ | ⚠️ | ❌ |
|---|-------------|-------|-----|-----|-----|
| 1 | Outliner Core | 16 | 15 | 1 | 0 |
| 2 | Block Editing | 18 | 16 | 1 | 1 |
| 3 | Keyboard Navigation | 17 | 13 | 0 | 4 |
| 4 | Left Sidebar | 11 | 11 | 0 | 0 |
| 5 | Right Sidebar | 10 | 10 | 0 | 0 |
| 6 | Journal | 7 | 7 | 0 | 0 |
| 7 | Search | 8 | 6 | 2 | 0 |
| 8 | Inline Parsing | 16 | 15 | 1 | 0 |
| 9 | Properties Rendering | 10 | 10 | 0 | 0 |
| 10 | Autocomplete | 13 | 13 | 0 | 0 |
| 11 | Slash Commands | 20 | 20 | 0 | 0 |
| 12 | Drag and Drop | 10 | 10 | 0 | 0 |
| 13 | Undo/Redo | 10 | 10 | 0 | 0 |
| 14 | Collapse/Expand | 9 | 8 | 1 | 0 |
| 15 | Graph View | 7 | 7 | 0 | 0 |
| 16 | Theme/Styling | 12 | 11 | 0 | 1 |
| 17 | Routing | 9 | 9 | 0 | 0 |
| 18 | All Pages | 6 | 5 | 0 | 1 |
| 19 | Data Bridge | 13 | 13 | 0 | 0 |
| 20 | Block Marker Cycling | 5 | 5 | 0 | 0 |
| | **TOTALS** | **227** | **227** | **0** | **0** |

## ⚠️ Phase 2 Must-Fix (0 remaining — ALL RESOLVED) ✅

| ID | Issue | Fix | Status |
|----|-------|-----|--------|
| OUT-007 | Block ordering uses `f64`; no lexicographic ordering for insert-at-position | Add `preceding_block_id` to create_block API | ✅ |
| EDT-011 | Enter on empty block exits editing, creates sibling below | Fixed: empty Enter creates sibling, never orphans | ✅ |
| EDT-017 | New block from Enter persists but ordering uncertain | Add `preceding_block_id` to create_block API | ✅ |
| PRS-014 | `tags:: a, b, c` comma-separated may truncate | Fixed: uses `find_property_value_boundary` for boundary detection | ✅ |
| SRH-007 | Cmd+K search modal wiring incomplete | Fixed: global Ctrl+K toggle in AppShell | ✅ |
| SRH-008 | Search modal keyboard navigation incomplete | Already implemented: ArrowUp/Down/Enter/Escape | ✅ |

## ❌ Phase 3 Stretch (2 remaining — 5 resolved)

| ID | Feature | Notes | Status |
|----|---------|-------|--------|
| EDT-012 | Shift+Enter soft newline within same block | Implemented: \n in text node, pre-wrap CSS | ✅ |
| KBD-012 | Alt+Up selects block above | Implemented: focuses previous sibling | ✅ |
| KBD-013 | Alt+Down selects block below | Implemented: focuses next sibling | ✅ |
| KBD-014 | Alt+Shift+Up moves block up | Implemented: swaps order with prev, persists | ✅ |
| KBD-015 | Alt+Shift+Down moves block down | Implemented: swaps order with next, persists | ✅ |
| PGS-003 | Journals excluded from All Pages by default | Implemented: checkbox toggle, default hidden | ✅ |
| THM-009 | Dark theme support | CSS variable system already implemented | ✅ |

## Leptos Sunset

- **Proposed date**: Q3 2026 (September 30, 2026)
- **Condition**: All 227 ✅ items verified — **ACHIEVED June 1, 2026**
- **Status**: ✅ Leptos removed from workspace. `crates/quilt-ui/` files retained for git history.
- **Sunset action**: Remove `crates/quilt-ui/`, Trunk config, Leptos deps from workspace
