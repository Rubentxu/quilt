# Logseq Block Interaction Specification

> **Source of truth** for replicating Logseq's block interaction model in Quilt.
> Derived from Logseq source code (`src/main/frontend/handler/editor.cljs`,
> `src/main/frontend/modules/shortcut/config.cljs`, `deps/outliner/`) and
> verified through runtime testing.

---

## 1. Block Editing Model

### 1.1 Click on block content → Enter edit mode
| Aspect | Behavior |
|--------|----------|
| Trigger | `click` on `.cursor-text` or block content area |
| Result | Block enters edit mode, editor receives focus immediately |
| Cursor | At click position within text |
| Logseq source | `block.cljs` → `bullet-on-click` / `editor-handler/edit-block!` |

### 1.2 Click on bullet
| Aspect | Behavior |
|--------|----------|
| Trigger | `click` on bullet circle (`.bullet-container`) |
| Result | Selects block (shows context menu). If block has children, toggles collapse. |
| Does NOT | Enter edit mode |
| Logseq source | `block.cljs` → `bullet-on-click` → collapse toggle |

### 1.3 Click outside block → Save + exit edit mode
| Aspect | Behavior |
|--------|----------|
| Trigger | `click` anywhere outside the editing block |
| Result | Current block content is saved to DB. Edit mode exits. |
| Important | Must persist BEFORE editor unmounts |
| Edge case | Click on another block's content → saves current, enters edit on new |

### 1.4 Blur → Save
| Aspect | Behavior |
|--------|----------|
| Trigger | `focusout` / `blur` on editor |
| Result | Save content, exit edit mode |
| Edge case | Should not double-save if click handler already saved |

### 1.5 Reload → Content persists
| Aspect | Behavior |
|--------|----------|
| After | Page reload / navigation away and back |
| Result | All saved content is visible |

---

## 2. Keyboard Shortcuts — Editing Mode

### 2.1 Enter — New block / Split

| Scenario | Behavior |
|----------|----------|
| **Enter at end of block** | Creates new **sibling** block BELOW current. Cursor moves to new block. New block is empty, enters edit mode. |
| **Enter in middle of text** | **Splits** block at cursor. Text before cursor stays in current block. Text after cursor moves to new sibling block below. Cursor moves to new block. |
| **Enter on empty block** | Exit edit mode on current block, create new sibling below. |
| **Enter with text selection** | Selection becomes new block content. Cursor moves to new block. |
| **Enter during autocomplete** | `autocomplete/complete` consumes Enter, selects suggestion. |
| Enter keybinding | `"enter"` → `editor-handler/keydown-new-block-handler` |
| Shift+Enter | **Soft newline** within the SAME block. Does NOT create new block. Inserts `\n` character. |

**Important ordering rules for new sibling:**
- New block goes AFTER current block (as right sibling)
- Uses `block/order` lexicographic string for positioning
- Uses `outliner-insert-block!` with `{:sibling? true}`

### 2.2 Tab — Indent (make child)

| Scenario | Behavior |
|----------|----------|
| **Tab on non-empty block** | Indents block to become **child** of the block ABOVE it. |
| **Tab on empty block** | Indents (same behavior) |
| **Tab at max indent** | No operation |
| Tab keybinding | `"tab"` → `editor-handler/keydown-tab-handler :right` |
| Indent logic | `move-blocks!` with `{:sibling? false}` (as child of previous) |

### 2.3 Shift+Tab — Outdent (make sibling of parent)

| Scenario | Behavior |
|----------|----------|
| **Shift+Tab on indented block** | Outdents block to become **sibling** of its current parent. |
| **Shift+Tab at level 1** | No operation |
| Shift+Tab keybinding | `"shift+tab"` → `editor-handler/keydown-tab-handler :left` |

### 2.4 Backspace — Delete / Merge

| Scenario | Behavior |
|----------|----------|
| **Backspace at start of empty block** | **Deletes** current empty block. Focus moves to previous block (end of content). |
| **Backspace at start of non-empty block** | **Merges** current block's content with end of previous block. Current block is deleted. Children of deleted block are re-parented. |
| **Backspace mid-text** | Normal character deletion |
| Backspace keybinding | `"backspace"` → `editor-handler/editor-backspace` |
| Merge children behavior | Deleted block's children become children of the merged block, preserving order |

### 2.5 Delete — Forward delete

| Scenario | Behavior |
|----------|----------|
| **Delete at end** | Merges with NEXT block (if exists) |
| **Delete mid-text** | Normal character deletion |
| Delete keybinding | `"delete"` → `editor-handler/editor-delete` |

### 2.6 Escape — Exit editing

| Scenario | Behavior |
|----------|----------|
| **Escape while editing** | Exits edit mode. If content changed, **SAVES** first. |
| **Escape after editing** | Clears editor state |
| Escape keybinding | `editor/escape-editing` → `editor-handler/escape-editing` |

### 2.7 Ctrl+Enter — Cycle TODO/DOING/DONE

| Scenario | Behavior |
|----------|----------|
| **Ctrl+Enter (Cmd+Enter on Mac)** | Cycles block marker: None → TODO → DOING → DONE → None |
| Works on | Currently editing block OR selected blocks |
| Keybinding | `"mod+enter"` → `editor-handler/cycle-todo!` |

### 2.8 Arrow Keys (during editing)

| Key | Behavior |
|-----|----------|
| **Up/Down** | Navigate between blocks while in edit mode. If at first/last line of block, moves to previous/next block. |
| **Left/Right** | Standard cursor movement within text |
| **Alt+Up** | Select block above (extends selection) |
| **Alt+Down** | Select block below (extends selection) |
| **Alt+Shift+Up** | Move current block up (reorder) |
| **Alt+Shift+Down** | Move current block down (reorder) |
| **Mod+Up** | Collapse block children |
| **Mod+Down** | Expand block children |

---

## 3. Keyboard Shortcuts — Non-Editing (Block Selected)

### 3.1 Arrow navigation
| Key | Behavior |
|-----|----------|
| **Up/Down** | Move selection to previous/next block |
| **Left** | Collapse selected block (if expanded and has children) |
| **Right** | Expand selected block (if collapsed) OR Enter edit mode on first child |
| **Enter** | Enter edit mode on selected block (at end of content) |

### 3.2 Block operations (non-editing)
| Key | Behavior |
|-----|----------|
| **Tab** | Indent selected block |
| **Shift+Tab** | Outdent selected block |
| **Backspace/Delete** | Delete selected block(s) |
| **Ctrl+C** | Copy selected block(s) (with children) |
| **Ctrl+X** | Cut selected block(s) |
| **Ctrl+V** | Paste block(s) |
| **Ctrl+Shift+V** | Paste as plain text |
| **Ctrl+A** | Select parent block |
| **Ctrl+Shift+A** | Select all blocks on page |

### 3.3 Text formatting (during editing)
| Key | Behavior |
|-----|----------|
| **Ctrl+B (Cmd+B)** | Bold selected text / toggle bold |
| **Ctrl+I (Cmd+I)** | Italic selected text / toggle italic |
| **Ctrl+Shift+H** | Highlight |
| **Ctrl+Shift+S** | Strikethrough |
| **Ctrl+L (Cmd+L)** | Insert link |
| **Ctrl+O (Cmd+O)** | Follow link under cursor |
| **Ctrl+Shift+O** | Open link in sidebar |
| **Ctrl+K (Cmd+K)** | Global search |

---

## 4. Mouse Interactions

### 4.1 Block content area
| Action | Behavior |
|--------|----------|
| **Single click** | Enter edit mode at click position |
| **Double click** | Select word |
| **Triple click** | Select all content in block |
| **Right click** | Context menu (cut, copy, paste, delete, etc.) |

### 4.2 Bullet area
| Action | Behavior |
|--------|----------|
| **Single click** | Select block + toggle collapse if has children |
| **Shift+click** | Multi-select range |
| **Drag bullet** | Drag-and-drop to reorder/reparent |
| **Right click bullet** | Context menu |

### 4.3 Whitespace between blocks
| Action | Behavior |
|--------|----------|
| **Double click** | Create new block at that position (not implemented in Logseq, desirable for Quilt) |

### 4.4 Drag and drop
| Action | Behavior |
|--------|----------|
| **Drag bullet left/right** | Change indent level (reparent) |
| **Drag bullet up/down** | Reorder among siblings |
| **Drag block (whole)** | Same as bullet drag |
| **Drop indicator** | Visual line shows where block will land |

---

## 5. Special Block Behaviors

### 5.1 Collapse/Expand
| Action | Behavior |
|--------|----------|
| **Click bullet (block with children)** | Toggle collapse |
| **Left arrow (non-editing, expanded)** | Collapse block |
| **Right arrow (non-editing, collapsed)** | Expand block |
| **Ctrl+Up (Cmd+Up)** | Collapse all children |
| **Ctrl+Down (Cmd+Down)** | Expand all children |
| **Ctrl+; (Cmd+;)** | Toggle collapse all |
| Visual indicator | Bullet changes: ○ (collapsed), ● (expanded with children) |

### 5.2 Block references `[[]]`
| Action | Behavior |
|--------|----------|
| **Type `[[`** | Triggers page reference autocomplete |
| **Type `((`** | Triggers block reference autocomplete |
| **Type `#`** | Triggers tag autocomplete |
| Autocomplete | Dropdown with filtered suggestions. Arrow keys navigate. Enter selects. |

### 5.3 Slash commands `/`
| Action | Behavior |
|--------|----------|
| **Type `/`** at start of block | Opens slash command menu |
| **Type `/` mid-block** | Opens slash command menu |
| Commands | TODO, DOING, DONE, LATER, NOW, Date picker, Upload asset, etc. |

### 5.4 Properties `::`
| Property format | `key:: value` in block content |
| Behavior | Rendered as styled property in display mode |
| Common props | `id::`, `tags::`, `alias::`, `template::`, `collapsed::`, `public::`, `icon::` |

---

## 6. Block Operations (programmatic)

### 6.1 Outliner operations
| Operation | Function |
|-----------|----------|
| save-block | `outliner-save-block!` → `outliner-op/save-block!` |
| insert-blocks | `outliner-insert-block!` → `outliner-op/insert-blocks!` |
| delete-blocks | `delete-block-inner!` → `outliner-op/delete-blocks!` |
| move-blocks | `move-blocks!` → `outliner-op/move-blocks!` |
| indent/outdent | `indent-outdent-blocks!` |
| cycle-todos | `cycle-todo!` |

### 6.2 Block ordering
| Property | Type | Description |
|----------|------|-------------|
| `block/order` | string | Lexicographically sortable key (e.g., "0a", "0b", "0c") |
| `block/parent` | ref | Parent block or page |
| `block/page` | ref | Owning page |
| Order generation | `db-order/gen-n-keys` creates keys between two existing keys |

---

## 7. Edge Cases

### 7.1 Empty block handling
- Backspace on empty block → delete block, move to previous
- Enter on empty block → exit editing, create new sibling
- Empty blocks are NOT automatically deleted on save

### 7.2 Paste behavior
- Paste multi-line text → creates multiple blocks (one per line)
- Paste at block start → inserts new blocks above current
- Paste at block end → inserts new blocks below current
- Paste with selection → replaces selection

### 7.3 Navigation safety
- Navigation away while editing → save current block before navigating
- Route change → should trigger save
- Tab close → browser warns if unsaved changes (Logseq does NOT do this)

### 7.4 Concurrent edits
- Only ONE block can be edited at a time
- Starting edit on new block → saves previous editing block first

---

## 8. Quilt Implementation Status

### Phase 0: Implemented ✅
| Behavior | Status |
|----------|--------|
| Click block content → enter edit mode | ✅ |
| Editor gets focus immediately | ✅ |
| Blur saves content to backend | ✅ |
| Reload persists content | ✅ |
| Click empty journal → create first block | ✅ |
| Enter creates new block (split at cursor) | ✅ |
| Indent/outdent (basic) | ✅ |
| Shift+Enter (soft newline) | ⚠️ Not yet |

### Phase 1: High Priority (Needed)
| Behavior | Priority | Notes |
|----------|----------|-------|
| **Backspace merge** at start of block | P0 | Critical for outliner feel |
| **Delete merge** at end of block | P0 | |
| **Tab indent** with proper parent tracking | P0 | Backend needs `block/order` |
| **Shift+Tab outdent** with proper re-parenting | P0 | |
| **Escape to exit** editing (save first) | P1 | |
| **Arrow keys** navigate between blocks | P1 | |
| **Ctrl+Enter** cycle TODO | P1 | Marker cycling already partially exists |
| **Click bullet** to select + collapse | P1 | Bullet exists but needs collapse behavior |

### Phase 2: Polish
| Behavior | Priority | Notes |
|----------|----------|-------|
| **Drag and drop** blocks | P2 | |
| **Collapse/Expand** with arrow keys | P2 | |
| **Soft newline Shift+Enter** | P2 | |
| **Block references autocomplete** | P2 | `[[` trigger already exists |
| **Copy/Cut/Paste** blocks | P2 | |
| **Undo/Redo** outliner-level | P2 | Outliner history exists |
| **Move block up/down** (Alt+Shift+Arrows) | P3 | |
| **Multi-select** blocks (Shift+click) | P3 | |

---

## 9. Backend Requirements for Full Parity

### 9.1 Block ordering
```rust
// Required: lexicographic order string
pub struct CreateBlockRequest {
    pub page_name: String,
    pub content: String,
    pub parent_id: Option<String>,
    pub preceding_block_id: Option<String>,  // For ordering
    pub order: Option<String>,               // Lexicographic order key
}
```

### 9.2 Block operations
- `POST /api/v1/blocks` — with ordering
- `PATCH /api/v1/blocks/:id` — content update ✅ (exists)
- `DELETE /api/v1/blocks/:id` — with children
- `POST /api/v1/blocks/:id/move` — reparent + reorder

### 9.3 Order generation algorithm
```rust
/// Generate a lexicographic key between two existing keys.
/// Uses a base-36-like system for compact string representation.
fn gen_order_key(prev: Option<&str>, next: Option<&str>) -> String {
    // Implementation: pick a midpoint string between prev and next
    // e.g., between "0a" and "0c" → "0b"
}
```

---

## Appendix: Logseq Keybinding Summary

| Category | Key | Action |
|----------|-----|--------|
| **Editing** | Enter | New block / split |
| | Shift+Enter | Soft newline (same block) |
| | Tab | Indent |
| | Shift+Tab | Outdent |
| | Backspace | Backspace (merge/delete at start) |
| | Delete | Forward delete (merge at end) |
| | Escape | Exit editing (save) |
| | Ctrl+Enter | Cycle TODO/DOING/DONE |
| **Navigation** | Up/Down | Navigate blocks |
| | Left/Right | Collapse/expand or cursor |
| | Ctrl+Up | Collapse children |
| | Ctrl+Down | Expand children |
| | Alt+Up/Down | Select block above/below |
| | Alt+Shift+Up/Down | Move block up/down |
| **Formatting** | Ctrl+B | Bold |
| | Ctrl+I | Italic |
| | Ctrl+L | Insert link |
| | Ctrl+Shift+H | Highlight |
| | Ctrl+Shift+S | Strikethrough |
| **Block ops** | Ctrl+C | Copy |
| | Ctrl+X | Cut |
| | Ctrl+V | Paste |
| | Ctrl+Shift+V | Paste raw |
| | Ctrl+A | Select parent |
| | Ctrl+Shift+A | Select all |
| | Ctrl+Z | Undo |
| | Ctrl+Shift+Z | Redo |
| **Global** | Ctrl+K | Search |
| | Ctrl+Shift+P | Command palette |
| | gj | Go to journals |
| | ga | Go to all pages |
| | gt | Go to tomorrow |
| | gn | Next journal |
| | gp | Previous journal |
