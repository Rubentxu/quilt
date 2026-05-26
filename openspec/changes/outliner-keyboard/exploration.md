# Exploration: Outliner Keyboard Handling for quilt-ui

## Topic
`outliner-keyboard` — Full keyboard-driven block editing for the quilt-ui outliner component

## Current State

### 1. quilt-ui Component Structure

**Existing components** (`crates/quilt-ui/src/`):

| File | Purpose |
|------|---------|
| `components/block.rs` | Block display with edit toggle — uses `editing` signal to swap between display and BlockEditor |
| `components/block_editor.rs` | **Current focus** — contenteditable div with basic keydown handling |
| `outliner/tree.rs` | `BlockNode` struct + `build_tree()` / `flatten_tree()` — converts flat `BlockDto` list to hierarchy |
| `bridge.rs` | `BlockDto` struct + HTTP client (`get_page_blocks`, `create_block`, `update_block`) |

**Current BlockEditor keyboard handling** (block_editor.rs:22-39):
```rust
let handle_keydown = move |ev: leptos::ev::KeyboardEvent| {
    match key.as_str() {
        "Enter" => { ev.prevent_default(); on_save(clone)(content.get()); }
        "Escape" => { ev.prevent_default(); content.set(block.get().content.clone()); on_cancel(clone)(); }
        "Tab" => { ev.prevent_default(); }  // ← hooked but does nothing
        _ => {}
    }
};
```

### 2. Leptos 0.8 Keyboard Events

- **`on:keydown`** — fires before browser default (use `ev.prevent_default()` to intercept)
- **`on:keyup`** / **`on:keypress`** — also available but keydown is sufficient
- **Modifier detection**: `ev.shift_key()`, `ev.ctrl_key()`, `ev.alt_key()`, `ev.meta_key()` — inherited from web-sys `KeyboardEvent`
- **IME handling**: `keydown` fires for physical keys; for IME composition, need `on:compositionstart` / `on:compositionend`
- **Cursor position**: Must use DOM APIs (`window.getSelection()`) since Leptos doesn't expose cursor state
- **Focus**: `node_ref` + `el.focus()` in Effect; `contenteditable="true"` for text editing

### 3. Block Data Model

**BlockDto** (bridge.rs:9-24) — UI transport type:
```rust
pub struct BlockDto {
    pub id: String,
    pub page_id: String,
    pub parent_id: Option<String>,
    pub content: String,
    pub order: f64,        // fractional indexing for sibling ordering
    pub level: u8,         // 1-indexed indent depth
    pub marker: Option<String>,
    pub collapsed: bool,
    pub properties: serde_json::Value,
    // ...
}
```

**Block entity** (quilt-domain/src/entities/block.rs) — domain type with `BlockUpdate` for mutations.

**OutlinerService** (quilt-domain/src/services/outliner_service.rs) provides:
- `calculate_order()` — fractional indexing for insert between siblings
- `rebalance_children()` — normalize fragmented orders
- `validate_move()` — circular reference check
- `can_move_to()` — block-level move eligibility

### 4. Existing Bridge API

Current HTTP endpoints in `bridge.rs`:
| Function | Method | Purpose |
|----------|--------|---------|
| `get_page_blocks` | GET | Fetch all blocks for a page |
| `create_block` | POST | Create new block |
| `update_block` | PATCH | Update block content |

**Missing for keyboard ops**: No API for `indent`, `outdent`, `delete_block`, `move_block`, or `merge_blocks`.

---

## Affected Areas

| File | Why Affected |
|------|--------------|
| `crates/quilt-ui/src/components/block_editor.rs` | Primary implementation site for keyboard handlers |
| `crates/quilt-ui/src/components/block.rs` | Parent component that owns editing state; needs to pass cursor position |
| `crates/quilt-ui/src/outliner/tree.rs` | Already handles tree building; needs new `indent()`, `outdent()`, `split_block()`, `merge_blocks()` operations |
| `crates/quilt-ui/src/bridge.rs` | Needs new API methods for `delete_block`, `move_block`, and batch operations |
| `crates/quilt-domain/src/entities/block.rs` | Block entity supports `parent_id` + `order` update via `BlockUpdate` |
| `crates/quilt-domain/src/services/outliner_service.rs` | Domain logic for order calculation exists; needs expand for indent/outdent |
| `crates/quilt-application/src/commands.rs` | Has `delete()` but no `indent`/`outdent`/`move` commands |

---

## Approaches

### Approach A: Pure Client-Side State + Server Sync

Handle all keyboard operations in UI state first (optimistic), then sync to server.

**Implementation**:
- Block state in `RwSignal<Vec<BlockDto>>` at page level
- Each `BlockEditor` receives callbacks: `on_enter`, `on_tab`, `on_shift_tab`, `on_backspace`, `on_split`, `on_merge`
- Operations mutate local state immediately, fire async server call in background
- Rollback on server error

**Pros**: Fast UX, works offline, simple mental model
**Cons**: Conflict resolution needed for concurrent edits; more state to manage

### Approach B: Server-First with Optimistic Fallback

Each keyboard operation calls server first, then updates UI on success.

**Implementation**:
- Keyboard handler calls `bridge::*` async function
- Server validates + executes operation, returns new block state
- UI updates from server response

**Pros**: Simpler consistency model, server is source of truth
**Cons**: Network latency on every keypress; poor UX if server is slow

### Approach C: Hybrid — Client Tree Mutations + Server Validation

Build tree operations in UI (already have flat list), batch-sync to server.

**Implementation**:
- Keyboard ops update local `BlockDto` list + tree
- Debounce server sync (e.g., 500ms after last change)
- Full tree state sent to server for validation

**Pros**: Balance of responsiveness and consistency
**Cons**: More complex sync logic; need conflict resolution

---

## Recommendation

**Approach A (Pure Client-Side + Server Sync)** with the following modifications:
1. Local state lives at page level as `RwSignal<Vec<BlockDto>>`
2. Each `Block` component receives a `block_ref` (entity index) rather than clone
3. Keyboard ops are pure transformations on the local signal
4. Server sync happens async; errors trigger UI toast notification

This matches how Logseq/Roam work — local-first with async persistence.

---

## Implementation: Key-by-Key

### Enter Key
**Current behavior**: Saves content and exits editing mode.
**Desired behavior**: 
1. Get cursor position within contenteditable (via `window.getSelection()`)
2. If cursor at end + block not empty → create new block below with remaining content
3. If cursor in middle → split block at cursor; first half stays, second half becomes new block
4. If block empty → just create new block
5. New block gets focus

**Needs**:
- DOM cursor position API
- `create_block` API call (already exists in bridge)
- Tree re-render with new block in correct position

### Tab Key (Indent)
**Current behavior**: `ev.prevent_default()` only.
**Desired behavior**:
1. Get previous sibling (same parent, lower order)
2. If no previous sibling → cannot indent (first child)
3. Update block's `parent_id` to previous sibling's id
4. Recalculate `order` using `OutlinerService::calculate_order()` with previous sibling's children
5. Update `level` to parent's level + 1

**Needs**:
- `move_block` API (update parent_id + order)
- Server-side circular reference check (already in `Block::can_move_to`)

### Shift+Tab Key (Dedent)
**Current behavior**: Not handled.
**Desired behavior**:
1. If block has no parent → cannot dedent (already at root)
2. Move block to become sibling of its parent (same parent_id as parent)
3. Recalculate `order` to be after parent's other children
4. Update `level` accordingly

**Needs**: Same `move_block` API as indent.

### Backspace Key
**Current behavior**: Browser default (delete character).
**Desired behavior**:
1. If block has content → do nothing (browser handles it)
2. If block has children → do nothing (cannot delete block with children)
3. If block empty + has no children:
   - Delete block via API
   - Move focus to previous sibling (or parent if first child)
4. If cursor at start of block AND block not empty:
   - Merge block content with previous sibling
   - Delete current block
   - Move cursor to end of previous block

**Needs**:
- `delete_block` API (already exists in bridge: `DELETE /blocks/{id}`)
- `update_block` API for content merge
- Focus management

### Escape Key
**Current behavior**: Cancels editing, reverts content.
**Desired behavior**: Already working — blur/clear selection.

### Split Key (Ctrl+Enter)
Split block at cursor position into two blocks.

**Needs**: Same as Enter-split case above.

### Merge Key (Ctrl+Backspace)
Merge current block with next sibling.

**Needs**:
- Find next sibling
- Concatenate next block's content to current
- Delete next block via API

---

## Edge Cases

| Case | Expected Behavior |
|------|-------------------|
| First block, Backspace at start | Nothing happens (no previous sibling) |
| Last block, Enter at end | Create new empty block at end, focus it |
| Nested indent limit | No hard limit; `level` is u8 (max 255) |
| Delete block with children | BlockHasChildren error; do nothing |
| Merge at end of page | Nothing happens (no next sibling) |
| Empty root-level blocks | Allowed; user must not delete last block |
| Concurrent edits | Not handled in this change; local-first assumption |

---

## Open Questions Before Spec

1. **Split/Merge keystrokes**: Ctrl+Enter for split, Ctrl+Backspace for merge — or different?
2. **Optimistic vs server-first**: Which approach to use? (Recommendation: optimistic)
3. **Block deletion with children**: Should `delete_block` recursively delete children, or refuse?
4. **Undo/redo**: Should keyboard handlers support undo? (e.g., Ctrl+Z)
5. **Server API gaps**: Need to add `move_block`, `delete_block` (if not already in MCP server), and potentially batch update endpoints
6. **Cursor position persistence**: After indent/dedent, where should cursor land?
7. **New block focus**: After Enter, should new block be empty with cursor inside, or pre-filled?
8. **IME composition**: How to handle Enter during active IME composition? (usually ignore Enter during composition)

---

## Entropy Analysis (Connascence Landscape)

**Method**: Heuristic (no CogniCode data for this change yet)

| Component A | Component B | Connascence Type | I(bits) | Severity |
|------------|-------------|------------------|---------|----------|
| `block_editor.rs` | `block.rs` | Name (editing signal) | 0.32 | ✅ OK |
| `block.rs` | `bridge.rs` | Type (BlockDto) | 1.58 | ⚠️ Medium |
| `bridge.rs` | `outliner/tree.rs` | Meaning (BlockDto↔Block) | 2.1 | ⚠️ Medium |
| `block_editor.rs` | DOM API | Meaning (cursor APIs) | ~3.0 | ❌ High |
| Keyboard handlers | OutlinerService | Name (calculate_order) | 1.8 | ⚠️ Medium |

**Critical Pairs (I > 3.0 bits)**: 
- `block_editor.rs` ↔ DOM API for cursor position — hidden meaning connascence (how to get/restore cursor)

**Hidden Connascence (Meaning/Timing)**:
- Cursor position across re-renders — if editor re-mounts, cursor position is lost unless explicitly preserved
- Tree ordering vs. visual order — `order` field determines sort, but tree building assumes this is correct

**Coupling Score**: H_external ≈ 2.4 (medium-high — the block_editor touches bridge, block, outliner, and DOM)
**Estimation Method**: Heuristic
**Confidence**: estimated

---

## Effort Estimate Per Key

| Key | Complexity | Effort | Notes |
|-----|------------|--------|-------|
| Enter | Medium | 2-3h | Need cursor split logic + create block |
| Tab (indent) | Low | 1-2h | `move_block` with parent_id change |
| Shift+Tab (dedent) | Medium | 2h | Logic to find correct new position |
| Backspace | High | 3-4h | Multiple cases; focus management complex |
| Escape | None | 0h | Already working |
| Split (Ctrl+Enter) | Medium | 2h | Same as Enter-split; separate code path |
| Merge (Ctrl+Backspace) | Medium | 2-3h | Concatenate + delete + focus |
| **Total** | | **12-16h** | |

---

## Ready for Proposal

**Yes** — with these clarifications needed from user:
1. Confirm keystrokes for Split/Merge
2. Confirm optimistic vs server-first approach
3. Decision on block deletion with children behavior
4. Scope: Implement keyboard handlers only, or also add missing API endpoints?
