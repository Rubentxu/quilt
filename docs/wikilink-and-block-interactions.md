# Logseq Wikilink & Block Interaction Reference (Descriptive)

> **Purpose**: This document is **DESCRIPTIVE** — it records what Logseq actually does, derived from source code analysis of `handler/editor.cljs`. It exists to inform Quilt's prescriptive specs (`quilt-behavior-spec.md`, `quilt-keyboard-shortcuts.md`).
> **Status**: Complete — based on `handler/editor.cljs` (logseq v0.10.x).

---

## 1. Wikilink Types

Logseq recognizes three wikilink syntaxes in block text:

| Syntax | Type | Example | Resolved as |
|--------|------|---------|-------------|
| `[[Page]]` | Page reference | `[[Projects]]` | Page (creates if missing) |
| `((block-uuid))` | Block reference | `((abcdef12...))` | Block (opens in sidebar) |
| `#tag` | Tag | `#project` | Page (treated as page) |
| URL | External link | `https://...` | Opens in new tab |

---

## 2. Click Behavior (Read Mode)

In read mode (block not being edited), wikilinks are rendered as clickable elements:

- **`[[Page]]` click**: Navigates to that page. If page does not exist, Logseq creates it and navigates.
- **`((block-id))` click**: Opens the block in the sidebar (not the main content area). The block's parent page is determined and the block is shown in a sidebar panel.
- **`#tag` click**: Navigates to the tag page (tags are stored as pages in Logseq).
- **URL click**: Opens in new browser tab via `js/window.open`.

### Sidebar behavior
- Sidebar blocks are added via `state/sidebar-add-block!` with `:block` type.
- Sidebar pages are added via `state/sidebar-add-block!` with `:page` type.
- Multiple sidebar panels can be stacked (Logseq supports multiple sidebar panels).

---

## 3. Click Behavior (Edit Mode)

In edit mode, wikilinks are **plain text** — no click handling. The cursor can be inside or adjacent to a wikilink without triggering navigation. This is the critical design decision: **edit mode decouples pointer from navigation**.

**Navigation in edit mode is keyboard-only.**

### Why plain text?
Wikilinks in edit mode are rendered inside a contenteditable input. Making them clickable would require intercepting clicks on specific text ranges, which conflicts with text selection and cursor positioning. Logseq avoids this complexity by making edit mode purely text-oriented.

---

## 4. Keyboard Navigation in Edit Mode

### 4.1 `Mod+Enter` — Follow Link Under Cursor

Logseq shortcut: `mod+enter` (maps to `mod/enter` in shortcut config)

**Implementation** (`handler/editor.cljs` line 1122):

```clojure
(defn follow-link-under-cursor!
  []
  (when-let [page (get-nearest-page-or-url)]
    (when-not (string/blank? page)
      (p/do!
       (state/clear-editor-action!)
       (save-current-block!)
       (if (re-find url-regex page)
         (js/window.open page)
         (<follow-page-link! page))))))
```

**Algorithm** (`extract-nearest-link-from-text` line 1055):

1. Scan text for all three link types simultaneously (page pattern, block-ref pattern, tag pattern, URL pattern)
2. For each match, compute distance from cursor position:
   - If cursor inside match → distance = 0 (highest priority)
   - If cursor before match → `cursor_pos - start_pos`
   - If cursor after match → `end_pos - cursor_pos`
3. Sort matches by distance ascending, pick the nearest
4. Strip syntax markers:
   - `[[Page]]` → `Page`
   - `#tag` → `tag` (strip leading `#`)
   - `((block-id))` → block-id string
   - URL → unchanged

**Link type detection in `follow-link-under-cursor!`**:
- URL → `js/window.open page` (new tab)
- Page name → `<follow-page-link! page>` (navigate; creates if missing)
- Block-ref UUID → `<follow-page-link! page>` (navigates to page containing block)

### 4.2 `Mod+Shift+Enter` — Open Link in Sidebar

Logseq shortcut: `mod+shift+enter` (confirmed from `open-link-in-sidebar!` line 1133)

```clojure
(defn open-link-in-sidebar!
  []
  (when-let [page (get-nearest-page)]
    (let [page-name (string/lower-case page)
          block? (util/uuid-string? page-name)]
      (when-let [page (db/get-page page-name)]
        (if block?
          (state/sidebar-add-block! ... :block)
          (state/sidebar-add-block! ... :page))))))
```

**Differences from `Mod+Enter`**:
- Only handles pages/tags/block-refs — **no URL handling**
- Opens in sidebar instead of navigating main content
- Block-ref UUID is detected via `util/uuid-string?` and opened as `:block` type

### 4.3 `Mod+O` — Follow Link (alternative)

Logseq also binds `mod+o` as an alternative to `mod+enter` for following links (from shortcut config). Behavior is identical to `mod+enter` in this codebase.

---

## 5. Block Reference Click in Read Mode

### `open-block-in-sidebar!` (line 215)

```clojure
(defn open-block-in-sidebar!
  [block-id]
  (when block-id
    (when-let [block (db/entity ...)]
      (let [page? (nil? (:block/page block))]
        (state/sidebar-add-block! ... (if page? :page :block))))))
```

- If block has no `:block/page` → it's a page → open as `:page`
- Otherwise → it's a block → open as `:block`

**Key behavior**: Block refs in Logseq do NOT navigate the main content area. They only open in sidebar. The parent page's content is NOT replaced. This is different from page refs which DO navigate.

---

## 6. Tag Behavior

### `#tag` syntax detection (line 1928-1938)

When user types `#` at the start of a line or after whitespace:
```clojure
(state/set-editor-action! :page-search-hashtag)
```

This opens the page search autocomplete with `#` as the query, allowing the user to search/create pages with that tag.

### Tag navigation
- Clicking a `#tag` in read mode → navigates to the tag page
- `Mod+Enter` when cursor is inside/near `#tag` → extracts `tag` (strips `#`) and calls `<follow-page-link! tag>`
- Tags are stored as pages under a `#` namespace in Logseq's DB

---

## 7. Page Search Autocomplete (`[[`)

When user types `[[`:
```clojure
(= prefix page-ref/left-brackets)
→ (commands/handle-step [:editor/search-page])
→ (state/set-editor-action-data! {:pos ... :selected selected})
```

This opens page search autocomplete. The selected page is inserted as `[[Page Name]]`.

---

## 8. Enter Key Behavior

### `keydown-new-block-handler` (line 2259)
```clojure
(defn keydown-new-block-handler [^js e]
  (if (or (state/doc-mode-enter-for-new-line?)
          (inside-of-single-block (:node state)))
    (keydown-new-line)   ;; soft break
    (keydown-new-block state))) ;; new block
```

**Logseq distinguishes**:
- `Enter` → new block (structural operation)
- `Shift+Enter` → new line within block (text operation, via `keydown-new-line`)

In doc-mode or single-block context, `Enter` → new line instead.

---

## 9. TODO Cycling

### `cycle-todo!` (line 691)
```clojure
TODO → DOING → DONE → (nil = non-task)
```

Applies to the current block. Must be in edit mode or block must be selected. Uses `ui-outliner-tx/transact!` with `{:outliner-op :cycle-todos}`.

Logseq also has batch cycling via `cycle-todos!` (line 679) which cycles all selected blocks simultaneously.

---

## 10. Block Ref Copy

### `copy-current-block-ref` (line 3138)
Copies the block reference string `((uuid))` to clipboard for the current block.

---

## 11. Key Handler Dispatch

Logseq's keyboard handling follows a priority order:

1. **Autocomplete handlers** — if a dropdown is open (page search, block search, command menu), keystrokes are consumed by the autocomplete
2. **Formatting handlers** — `Mod+B`, `Mod+I`, etc.
3. **Shortcut-matched handlers** — `mod+enter`, `mod+shift+enter`, `mod+o`
4. **`keydown-not-matched-handler`** — catch-all for Enter, Tab, etc.

The `keydown-not-matched-handler` (line 2822) is the fallback dispatcher.

---

## 12. Autocomplete Closes When

From `close-autocomplete-if-outside` (line 1856):
- Cursor leaves `[[` wrapped context
- Cursor leaves `((` wrapped context
- Cursor leaves `#tag` wrapped context
- `Escape` is pressed

---

## 13. Quilt Gap Analysis

### 13.1 Already Implemented ✅

| Feature | Quilt Status | Evidence |
|---------|--------------|----------|
| `Mod+Enter` follow page link | ✅ Done | `BlockRow.tsx` `findNearestLink` + `Cmd/Ctrl+Enter` handler |
| `[[Page]]` click in read mode | ✅ Done | `InlineContent.tsx` `e.stopPropagation()` |
| `[[Page]]` page creation if missing | ✅ Done | `follow-link-under-cursor!` parity |
| Wikilink squarify (visual) | ✅ Done | `InlineContent.tsx` radius 2px, padding 1px 10px |

### 13.2 Not Yet Implemented ❌

| Feature | Quilt Status | Priority |
|---------|--------------|----------|
| `Mod+Shift+Enter` open in sidebar | ❌ Not implemented | HIGH — Logseq parity |
| `((block-id))` click in read mode | ❌ Not implemented | HIGH |
| `((block-id))` open in sidebar | ❌ Not implemented | HIGH — differs from page ref |
| `#tag` click in read mode | ❌ Not implemented | MEDIUM |
| `Mod+O` follow link | ❌ Not implemented (alternative to `Mod+Enter`) | LOW |
| `Escape` exit edit mode | ❌ Not implemented | MEDIUM |
| `Shift+Enter` soft break | ❌ Not implemented | MEDIUM |
| `Delete` forward delete | ❌ Not implemented | LOW |
| Block ref `copy-block-ref` | ❌ Not implemented | LOW |
| Multiple sidebar panels | ❌ Not implemented | MEDIUM |

### 13.3 Specific Discrepancies

1. **`((block-id))` navigation**: In Logseq, clicking block ref opens in **sidebar only**. In Quilt, block refs might currently navigate main content (same as page refs). This is a behavioral discrepancy.

2. **`Mod+Enter` on block-ref**: Logseq resolves block-ref to parent page and navigates there. Quilt's `findNearestLink` returns `type: 'block'` but the handler may not resolve to parent page.

3. **Sidebar stacking**: Logseq supports multiple sidebar panels. Quilt's sidebar implementation may be single-panel.

4. **`#tag` as page**: Logseq treats `#tag` identically to `[[Page]]` for navigation. Quilt may not handle `#tag` in the same way.

---

## 14. Cross-Reference

- **`quilt-keyboard-shortcuts.md`** §2.2: `Mod+O` listed as `FollowLink` (❌ unimplemented), `Mod+Shift+O` listed as `OpenInSidebar` (❌ unimplemented). Note: Logseq uses `mod+enter` and `mod+shift+enter` — not `mod+o`. The shortcuts doc should be updated to match Logseq's actual bindings.
- **`quilt-behavior-spec.md`** §1.1: Enter behavior matches Logseq. Soft break (`Shift+Enter`) is not yet implemented.
- **`quilt-behavior-spec.md`** §2: Block navigation (selected mode) needs `((block-id))` sidebar-open behavior documented as prescriptive spec.

---

## 15. Source File Locations

| Function | File | Line |
|----------|------|------|
| `follow-link-under-cursor!` | `handler/editor.cljs` | 1122 |
| `open-link-in-sidebar!` | `handler/editor.cljs` | 1133 |
| `open-block-in-sidebar!` | `handler/editor.cljs` | 215 |
| `extract-nearest-link-from-text` | `handler/editor.cljs` | 1055 |
| `get-nearest-page-or-url` | `handler/editor.cljs` | 1089 |
| `get-nearest-page` | `handler/editor.cljs` | 1100 |
| `<follow-page-link!` | `handler/editor.cljs` | 1111 |
| `cycle-todo!` | `handler/editor.cljs` | 691 |
| `keydown-new-block-handler` | `handler/editor.cljs` | 2259 |
| `keydown-new-line-handler` | `handler/editor.cljs` | 2270 |
| `copy-current-block-ref` | `handler/editor.cljs` | 3138 |
| `keydown-not-matched-handler` | `handler/editor.cljs` | 2822 |
| `close-autocomplete-if-outside` | `handler/editor.cljs` | 1856 |
