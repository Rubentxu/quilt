# Proposal: Outliner Keyboard Handling for quilt-ui

## Executive Summary

Implement full keyboard-driven block editing for the quilt-ui outliner. All 7 keyboard operations (Enter, Shift+Enter, Tab, Shift+Tab, Backspace, Escape, Ctrl+Enter, Ctrl+Backspace) work per Logseq behavior with optimistic UI updates and async server sync.

## Intent

Enable keyboard-only block editing in the outliner component matching Logseq's behavior:
- **Enter** — split block at cursor, create new block below
- **Shift+Enter** — insert newline within block
- **Tab** — indent block (become child of previous sibling)
- **Shift+Tab** — outdent block (become sibling of parent)
- **Backspace** — merge with previous sibling or delete empty block
- **Escape** — cancel editing, revert content
- **Ctrl+Enter** — explicit split at cursor
- **Ctrl+Backspace** — merge with next sibling

## Scope

### In
- New `keyboard_handlers.rs` component with all 7 key operations
- Modified `block_editor.rs` with cursor-aware split logic
- Modified `block.rs` with page-level state callbacks
- Modified `bridge.rs` with `delete_block`, `move_block` API methods
- Modified `outliner/tree.rs` with `indent()`, `outdent()`, `split_block()`, `merge_blocks()` operations
- Server-side API endpoints for `move_block` and `delete_block`

### Out
- Undo/redo (Ctrl+Z) — deferred to future change
- Concurrent edit conflict resolution — local-first assumption
- IME composition handling — deferred

## Approach: Pure Client-Side + Server Sync (Approach A)

```
KeyboardEvent → local RwSignal<Vec<BlockDto>> mutation (optimistic)
              → async bridge call
              → rollback on error + toast notification
```

**State**: Page-level `RwSignal<Vec<BlockDto>>` owned by parent, passed as callbacks to `BlockEditor`.

**Cursor**: DOM `window.getSelection()` for position; preserved across re-renders via `node_ref`.

## Entropy Budget (Protocol B)

### H(Δ_existing) — Modified Files

| File | Change Type | Coupling |
|------|-------------|----------|
| `block_editor.rs` | Add cursor APIs + handlers | High (→ DOM) |
| `block.rs` | Pass callbacks to BlockEditor | Medium |
| `bridge.rs` | Add `move_block`, `delete_block` | Medium |
| `outliner/tree.rs` | Add tree mutation ops | Medium |

### H(Δ_new) — New Components

| Component | LOC | Responsibility |
|-----------|-----|----------------|
| `keyboard_handlers.rs` | ~150 | Key event dispatch + cursor management |

### New Connascence Pairs

| Pair | Type | I(bits) | Mitigation |
|------|------|---------|------------|
| `block_editor` ↔ DOM | Meaning | ~3.0 | Document cursor API contract |
| `BlockDto` ↔ `OutlinerService` | Name | 1.8 | Use existing `calculate_order()` |
| `bridge` ↔ `outliner/tree` | Meaning | 2.1 | BlockDto is stable interface |

### OCP Compliance
- `keyboard_handlers.rs` — closed for modification, open for extension via new handler modules
- `outliner/tree.rs` — tree ops are additive, existing `build_tree`/`flatten_tree` unchanged

## Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| DOM cursor lost on re-render | High | High | `node_ref` + explicit focus restoration |
| Optimistic update conflicts | Medium | Medium | Toast on server error + rollback |
| Missing `move_block` API | High | Medium | Implement alongside keyboard handlers |
| Backspace at start of first block | Low | Low | No-op guard in handler |
| Delete block with children | Low | Low | BlockHasChildren check before API call |

## Rollback Plan

```bash
git revert HEAD -- "**/keyboard_handlers.rs" "**/block_editor.rs" "**/block.rs" "**/bridge.rs" "**/outliner/tree.rs"
```

All changes are additive; no schema migrations.

## Success Criteria

- [ ] Enter at end of block → new empty block below with focus
- [ ] Enter in middle of block → block splits at cursor; both halves correct
- [ ] Shift+Enter → newline inserted within block
- [ ] Tab → block indented as child of previous sibling
- [ ] Shift+Tab → block outdented to parent's level
- [ ] Backspace at start of block → content merges with previous sibling
- [ ] Backspace on empty block → block deleted, focus moves to previous sibling
- [ ] Escape → editing cancelled, content reverted
- [ ] Ctrl+Enter → explicit split (same as Enter mid-block)
- [ ] Ctrl+Backspace → merge with next sibling
- [ ] All operations sync to server asynchronously
- [ ] `cargo test` passes; `cargo clippy` clean
