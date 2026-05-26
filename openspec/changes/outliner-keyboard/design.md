# Design: Outliner Keyboard Handling

**Change**: `outliner-keyboard`
**Date**: 2026-05-24
**Status**: Draft

---

## 1. Executive Summary

Implement full keyboard-driven block editing for `quilt-ui` outliner. All 7 keyboard operations (Enter, Shift+Enter, Tab, Shift+Tab, Backspace, Escape, Ctrl+Enter, Ctrl+Backspace) work per Logseq behavior with optimistic UI updates and async server sync. Keyboard events flow: DOM → handler → local state (RwSignal) → bridge → server, with rollback on error.

---

## 2. Keyboard Event Flow

```
KeyboardEvent (keydown)
  └─> KeyboardHandlers::dispatch(key, modifiers, cursor_offset)
        ├─> Enter        ──> Block::on_enter(cursor_offset)
        │     └─> outliner::split_block() + bridge::create_block()
        ├─> Shift+Enter ──> [browser default — no intercept]
        ├─> Tab          ──> Block::on_tab()
        │     └─> outliner::indent() + bridge::move_block()
        ├─> Shift+Tab    ──> Block::on_shift_tab()
        │     └─> outliner::outdent() + bridge::move_block()
        ├─> Backspace    ──> Block::on_backspace(cursor_offset)
        │     ├─> cursor_offset == 0 ──> outliner::merge_blocks() + bridge::delete_block()
        │     └─> cursor_offset >  0 ──> outliner::merge_content() + bridge::update_block()
        ├─> Escape       ──> Block::on_cancel()
        │     └─> restore saved content + exit editing mode
        ├─> Ctrl+Enter   ──> Block::on_split(cursor_offset) [explicit split]
        └─> Ctrl+Backspace ──> Block::on_merge_next()
              └─> outliner::merge_with_next() + bridge::update_block() + bridge::delete_block()
```

**IME Composition**: When `key === "Compose"` or `ev.isComposing`, all Enter/Backspace events are ignored (no intercept).

---

## 3. Component Architecture

### 3.1 New File: `quilt-ui/src/components/keyboard_handlers.rs`

```rust
use leptos::prelude::*;

/// Keyboard modifier mask
#[derive(Debug, Clone, Copy, Default)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

/// Cursor position from DOM
#[derive(Debug, Clone, Copy)]
pub struct CursorOffset(pub u32);

/// Key dispatch result
#[derive(Debug)]
pub enum DispatchResult {
    Handled,      // Key was intercepted, prevent default
    Bubble,       // No handler matched, allow default
}

/// Dispatches keyboard events to appropriate handlers.
pub struct KeyboardHandlers {
    pub on_enter: Box<dyn Fn(u32) + Clone + 'static>,
    pub on_tab: Box<dyn Fn() + Clone + 'static>,
    pub on_shift_tab: Box<dyn Fn() + Clone + 'static>,
    pub on_backspace: Box<dyn Fn(u32) + Clone + 'static>,
    pub on_escape: Box<dyn Fn() + Clone + 'static>,
    pub on_ctrl_enter: Box<dyn Fn(u32) + Clone + 'static>,
    pub on_ctrl_backspace: Box<dyn Fn() + Clone + 'static>,
}

impl KeyboardHandlers {
    /// Dispatch a keydown event. Returns whether default was prevented.
    pub fn dispatch(
        &self,
        key: &str,
        modifiers: Modifiers,
        cursor_offset: u32,
        is_composing: bool,
    ) -> DispatchResult {
        if is_composing {
            return DispatchResult::Bubble;
        }

        match (key, modifiers.shift, modifiers.ctrl) {
            ("Enter", false, false) => {
                (self.on_enter)(cursor_offset);
                DispatchResult::Handled
            }
            ("Enter", true, false) => DispatchResult::Bubble,
            ("Tab", false, false) => {
                (self.on_tab)();
                DispatchResult::Handled
            }
            ("Tab", true, false) => {
                (self.on_shift_tab)();
                DispatchResult::Handled
            }
            ("Backspace", false, false) => {
                (self.on_backspace)(cursor_offset);
                DispatchResult::Handled
            }
            ("Escape", _, _) => {
                (self.on_escape)();
                DispatchResult::Handled
            }
            ("Enter", false, true) => {
                (self.on_ctrl_enter)(cursor_offset);
                DispatchResult::Handled
            }
            ("Backspace", false, true) => {
                (self.on_ctrl_backspace)();
                DispatchResult::Handled
            }
            _ => DispatchResult::Bubble,
        }
    }
}

/// Read cursor offset from DOM selection.
pub fn get_cursor_offset(container: &web_sys::HtmlElement) -> u32 {
    let selection = window()
        .get_selection()
        .ok()
        .flatten()
        .and_then(|s| s.anchor_offset() as u32);
    selection.unwrap_or(0)
}

/// Set cursor position in DOM.
pub fn set_cursor(container: &web_sys::HtmlElement, offset: u32) {
    if let Some(selection) = window().get_selection().ok().flatten() {
        let range = window().document()
            .and_then(|d| d.create_range().ok())
            .expect("createRange");
        // ... set start/end at text offset within container
    }
}
```

### 3.2 Modified: `block_editor.rs`

**Current state**: Minimal contenteditable with `on_save`, `on_cancel` callbacks.

**Changes**:

```rust
#[component]
pub fn BlockEditor(
    #[prop(into)] block: Signal<BlockDto>,
    // New callback props (all optional, defaults to no-op)
    on_enter: Option<Box<dyn Fn(u32) + Clone + 'static>>,
    on_tab: Option<Box<dyn Fn() + Clone + 'static>>,
    on_shift_tab: Option<Box<dyn Fn() + Clone + 'static>>,
    on_backspace: Option<Box<dyn Fn(u32) + Clone + 'static>>,
    on_escape: Option<Box<dyn Fn() + Clone + 'static>>,
    on_ctrl_enter: Option<Box<dyn Fn(u32) + Clone + 'static>>,
    on_ctrl_backspace: Option<Box<dyn Fn() + Clone + 'static>>,
    // Internal cursor tracking
    cursor_offset: Option<StoredValue<u32>>,
) -> impl IntoView
```

**Cursor Preservation**: On every render, an `Effect` restores focus and cursor position using `node_ref`:

```rust
Effect::new(move || {
    if let Some(el) = el_ref.get() {
        let _ = el.focus();
        if let Some(offset) = cursor_offset.and_then(|c| c.get()) {
            set_cursor(&el, offset);
        }
    }
});
```

**Key handler** (`handle_keydown` in current code) is replaced with dispatch to `KeyboardHandlers` and IME composition check.

### 3.3 Modified: `block.rs`

**Current state**: Owns `editing` signal, renders `BlockEditor` with `on_save`, `on_cancel`.

**Changes**:

1. `Block` receives page-level `RwSignal<Vec<BlockDto>>` and `bridge::Bridge` handle as context or props.
2. All keyboard callbacks are wired from `BlockEditor` → `Block`.
3. On `on_enter`/`on_tab`/etc., `Block` reads page signal, applies tree mutation locally, then calls bridge.
4. On error, rollback local state + show toast.

```rust
#[component]
pub fn Block(
    #[prop(into)] block: Signal<BlockDto>,
    #[prop(into)] blocks: RwSignal<Vec<BlockDto>>,  // page-level state
    // keyboard callbacks passed to BlockEditor
) -> impl IntoView {
    let (editing, set_editing) = signal(false);
    let original_content = StoredValue::new(block.get().content.clone());

    // BlockEditor receives merged callbacks
    let keyboard_handlers = KeyboardHandlers {
        on_enter: Box::new(move |offset| { /* ... */ }),
        on_tab: Box::new(move || { /* ... */ }),
        // ...
    };

    let on_cancel = move || {
        content.set(original_content.get());
        set_editing.set(false);
    };
}
```

### 3.4 Modified: `bridge.rs`

**Current state**: `BlockDto`, `get_page_blocks`, `create_block`, `update_block`.

**New functions**:

```rust
/// Error types for block operations.
#[derive(Debug, Clone)]
pub enum BlockError {
    Network(String),
    Parse(String),
    Server(u16, String),
    BlockNotFound(String),
    BlockHasChildren(String),
    ConcurrentEdit(String),
}

impl std::fmt::Display for BlockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { /* ... */ }
}

/// Delete a block. Fails if block has children.
pub async fn delete_block(block_id: &str) -> Result<(), BlockError> {
    let url = format!("{}/blocks/{}", BASE_URL, block_id);
    let resp = Request::delete(&url)
        .send()
        .await
        .map_err(|e| BlockError::Network(e.to_string()))?;
    if !resp.ok() {
        let msg = resp.text().await.unwrap_or_default();
        if resp.status() == 409 {
            return Err(BlockError::BlockHasChildren(block_id.to_string()));
        }
        return Err(BlockError::Server(resp.status().as_u16(), msg));
    }
    Ok(())
}

/// Move a block to a new parent (indent/outdent) or reorder.
pub async fn move_block(
    block_id: &str,
    new_parent_id: Option<&str>,
    new_order: f64,
) -> Result<BlockDto, BlockError> {
    let url = format!("{}/blocks/{}/move", BASE_URL, block_id);
    let body = serde_json::json!({
        "new_parent_id": new_parent_id,
        "order": new_order,
    });
    let resp = Request::put(&url)
        .json(&body)
        .map_err(|e| BlockError::Network(e.to_string()))?
        .send()
        .await
        .map_err(|e| BlockError::Network(e.to_string()))?;
    if !resp.ok() {
        return Err(BlockError::Server(resp.status().as_u16(), resp.text().await.unwrap_or_default()));
    }
    let block: BlockDto = resp.json().await.map_err(|e| BlockError::Parse(e.to_string()))?;
    Ok(block)
}
```

### 3.5 Modified: `outliner/tree.rs`

**Current state**: `build_tree`, `flatten_tree`, `BlockNode`, `count_descendants`.

**New functions**:

```rust
use crate::bridge::BlockDto;

/// Indent a block: make it the last child of its previous sibling.
pub fn indent(blocks: &mut Vec<BlockDto>, block_id: &str) -> Result<(), TreeError> {
    let idx = blocks.iter().position(|b| b.id == block_id)
        .ok_or(TreeError::BlockNotFound)?;
    let prev_idx = find_previous_sibling(blocks, idx)
        .ok_or(TreeError::NoPreviousSibling)?;

    let new_parent_id = blocks[prev_idx].id.clone();
    let new_order = blocks[prev_idx].order + 1.0; // Simplified; use OutlinerService::calculate_order

    blocks[idx].parent_id = Some(new_parent_id);
    blocks[idx].order = new_order;
    blocks[idx].level = blocks[prev_idx].level + 1;
    Ok(())
}

/// Outdent a block: make it a sibling of its parent.
pub fn outdent(blocks: &mut Vec<BlockDto>, block_id: &str) -> Result<(), TreeError> {
    let idx = blocks.iter().position(|b| b.id == block_id)
        .ok_or(TreeError::BlockNotFound)?;
    let block = &blocks[idx];
    let parent_id = block.parent_id.as_ref()
        .ok_or(TreeError::NoParent)?;

    let parent_idx = blocks.iter().position(|b| b.id == *parent_id)
        .ok_or(TreeError::ParentNotFound)?;
    let parent = &blocks[parent_idx];

    blocks[idx].parent_id = parent.parent_id.clone();
    blocks[idx].order = parent.order + 1.0;
    blocks[idx].level = parent.level;
    Ok(())
}

/// Split a block at cursor position. Returns (updated_block, new_block).
pub fn split_block(
    blocks: &mut Vec<BlockDto>,
    block_id: &str,
    cursor: u32,
) -> Result<(BlockDto, BlockDto), TreeError> {
    let idx = blocks.iter().position(|b| b.id == block_id)
        .ok_or(TreeError::BlockNotFound)?;

    let block = &blocks[idx];
    let (first, second) = split_content(&block.content, cursor as usize);

    let mut updated = blocks[idx].clone();
    updated.content = first;

    let new_block = BlockDto {
        id: uuid::Uuid::new_v4().to_string(),
        page_id: block.page_id.clone(),
        parent_id: block.parent_id.clone(),
        content: second,
        order: block.order + 0.5,
        level: block.level,
        marker: None,
        priority: None,
        collapsed: false,
        properties: serde_json::json!({}),
        refs: vec![],
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        created_by: None,
    };

    blocks[idx] = updated;
    blocks.insert(idx + 1, new_block.clone());
    Ok((blocks[idx].clone(), new_block))
}

/// Merge source block content into target at cursor position.
pub fn merge_content(
    blocks: &mut Vec<BlockDto>,
    target_id: &str,
    source_id: &str,
    cursor_offset: u32,
) -> Result<BlockDto, TreeError> {
    let target_idx = blocks.iter().position(|b| b.id == target_id)
        .ok_or(TreeError::BlockNotFound)?;
    let source_idx = blocks.iter().position(|b| b.id == source_id)
        .ok_or(TreeError::BlockNotFound)?;

    let combined = format!("{}{}", &blocks[target_idx].content, &blocks[source_idx].content);
    blocks[target_idx].content = combined;
    blocks.remove(source_idx);
    Ok(blocks[target_idx].clone())
}

/// Merge with next sibling.
pub fn merge_with_next(
    blocks: &mut Vec<BlockDto>,
    block_id: &str,
) -> Result<BlockDto, TreeError> {
    let idx = blocks.iter().position(|b| b.id == block_id)
        .ok_or(TreeError::BlockNotFound)?;
    let next_idx = idx + 1;
    if next_idx >= blocks.len() {
        return Err(TreeError::NoNextSibling);
    }
    merge_content(blocks, block_id, &blocks[next_idx].id, u32::MAX)
}

fn split_content(content: &str, cursor: usize) -> (String, String) {
    let mut chars = content.chars();
    let first: String = chars.by_ref().take(cursor).collect();
    let second: String = chars.collect();
    (first, second)
}

#[derive(Debug, Clone)]
pub enum TreeError {
    BlockNotFound,
    ParentNotFound,
    NoPreviousSibling,
    NoParent,
    NoNextSibling,
}

impl std::fmt::Display for TreeError { /* ... */ }
```

---

## 4. Data Flow

### 4.1 Optimistic Update Pattern

```
User presses Enter
  │
  ├─ 1. DOM captures keydown
  │
  ├─ 2. BlockEditor::handle_keydown → KeyboardHandlers::dispatch()
  │
  ├─ 3. (on_enter)(cursor_offset) callback fires
  │
  ├─ 4. Block reads page-level RwSignal<Vec<BlockDto>>
  │
  ├─ 5. Block calls outliner::split_block() ── mutates local Vec ── ✓
  │
  ├─ 6. Block calls bridge::create_block() ── async server call
  │
  ├─ 7. If server OK → done
  │     If server ERROR → rollback RwSignal + show toast
  │
  └─ 8. Effect restores focus/cursor to new block
```

### 4.2 Page-Level State Ownership

`PageView` (`page.rs`) owns `RwSignal<Vec<BlockDto>>`:

```rust
let (blocks, set_blocks) = signal(Vec::new());

// blocks is passed to Block component as RwSignal
<Block block=Signal::derive(move || block.clone()) blocks=blocks children=vec![] />
```

`Block` reads `blocks` to find siblings, calls `outliner::*` operations, then `set_blocks.update(...)` to apply changes.

---

## 5. Error Types

| Error | Domain | Bridge Variant | HTTP Status |
|-------|--------|----------------|-------------|
| `BlockNotFound` | Domain | `BlockError::BlockNotFound` | 404 |
| `BlockHasChildren` | Domain | `BlockError::BlockHasChildren` | 409 |
| `ConcurrentEdit` | Domain | `BlockError::ConcurrentEdit` | 409 |
| `Network` | — | `BlockError::Network` | — |
| `Server` | — | `BlockError::Server` | 5xx |

---

## 6. API Signatures

### Bridge (client-side)

```rust
pub async fn delete_block(block_id: &str) -> Result<(), BlockError>
pub async fn move_block(block_id: &str, new_parent_id: Option<&str>, new_order: f64) -> Result<BlockDto, BlockError>
```

### Tree Operations

```rust
pub fn indent(blocks: &mut Vec<BlockDto>, block_id: &str) -> Result<(), TreeError>
pub fn outdent(blocks: &mut Vec<BlockDto>, block_id: &str) -> Result<(), TreeError>
pub fn split_block(blocks: &mut Vec<BlockDto>, block_id: &str, cursor: u32) -> Result<(BlockDto, BlockDto), TreeError>
pub fn merge_content(blocks: &mut Vec<BlockDto>, target_id: &str, source_id: &str, cursor: u32) -> Result<BlockDto, TreeError>
pub fn merge_with_next(blocks: &mut Vec<BlockDto>, block_id: &str) -> Result<BlockDto, TreeError>
```

---

## 7. Server-Side Endpoints (assumed REST)

| Method | Path | Body | Response |
|--------|------|------|----------|
| `DELETE` | `/api/blocks/{id}` | — | 204 or 409 (BlockHasChildren) |
| `PUT` | `/api/blocks/{id}/move` | `{new_parent_id, order}` | BlockDto or 409 |

---

## 8. Focus Management

1. **`node_ref` on contenteditable div** — used to imperatively focus after render.
2. **`StoredValue<u32>` for cursor offset** — preserved across re-renders.
3. **`Effect`** runs after every render to restore both focus and cursor position.
4. **On split**: After new block inserts, focus moves to the new block's editor via `node_ref`.

---

## 9. Connascence Analysis

| Pair | Type | I(bits) | Mitigation |
|------|------|---------|------------|
| `block_editor` ↔ DOM | Meaning | ~3.0 | Document cursor API contract; `get_cursor_offset`/`set_cursor` wrappers |
| `BlockDto` ↔ `OutlinerService` | Name | 1.8 | Use existing `calculate_order()` |
| `bridge` ↔ `outliner/tree` | Meaning | 2.1 | `BlockDto` is stable interface |
| `block.rs` ↔ `RwSignal<Vec<BlockDto>>` | Position | 2.5 | Page-level signal doc; `set_blocks` atomic updates |

---

## 10. Risk Mitigation

| Risk | Mitigation |
|------|-----------|
| DOM cursor lost on re-render | `node_ref` + explicit `focus()` + cursor restoration Effect |
| Optimistic update conflicts | Toast on server error + atomic rollback of `RwSignal` |
| Missing `move_block` API | Implement alongside keyboard handlers (this change) |
| Backspace at start of first block | Guard: `if idx == 0` → no-op |
| Delete block with collapsed children | `BlockHasChildren` check before API call |
| IME composition bypass | `isComposing` flag check before dispatch |
