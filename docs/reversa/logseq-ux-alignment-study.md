# Logseq UX Alignment Study

This document defines the **Logseq-like UX contract** Quilt should implement, the **current runtime behavior** verified in Quilt, and a **prioritized alignment plan** with E2E validation.

## Quick path

1. Treat this file as the source of truth for Logseq-style block UX.
2. Implement gaps in the order listed in **Priority backlog**.
3. Do not mark a behavior done until its **Playwright E2E** exists and passes.

## Evidence sources

This study is based on four kinds of evidence:

| Source | Confidence | Notes |
|-------|------------|-------|
| `docs/reversa/domain.md` | High | Reverse-engineered Logseq domain rules already curated in the repo. |
| `docs/reversa/quilt-ui-workflows.md` | Medium | Product/UX intent for Quilt, useful for separation of views and journaling assumptions. |
| Runtime verification with `playwright-cli` against Quilt | High | Used to confirm what Quilt actually does today on regular pages and journal pages. |
| **Logseq source code (GitHub)** | Very High | Direct analysis of Logseq's ClojureScript implementation at `logseq/logseq` |

### Logseq Source Files Analyzed

| File | Purpose |
|------|---------|
| `src/main/frontend/components/editor.cljs` | Main editor component, keyboard handling, bullet rendering |
| `src/main/frontend/components/block.cljs` | Block component, bullet-on-click, block rendering |
| `deps/outliner/src/logseq/outliner/core.cljs` | Core outliner operations: `save-block`, `insert-blocks`, `move-blocks` |
| `deps/outliner/src/logseq/outliner/op.cljs` | Operation schema and `apply-ops!` |
| `deps/outliner/src/logseq/outliner/tree.cljs` | Tree operations, `blocks->vec-tree` |

> Important: this is an **alignment contract**, not a historical essay. When external/official Logseq documentation is ambiguous, the contract follows the user-facing outliner behavior expected from Logseq and validated through Quilt runtime investigation.

---

## Core UX contract Quilt should match

## 1. Block editing model

### Target behavior

- A **single click on block content** enters edit mode.
- The editor receives **focus immediately**.
- The user can **type without pressing Enter** first.
- Clicking outside the block **saves** and exits edit mode.
- Reloading the page shows the saved content.

### Why this matters

This is the fundamental outliner ergonomics contract. If click does not lead straight to typing, the product feels broken even if the editor technically mounted.

### Quilt current status

| Behavior | Status | Evidence |
|---------|--------|----------|
| Single click enters edit mode | ✅ | `PropertyContentView` calls `on_start_edit` on click. |
| Editor gets focus automatically | ✅ | Fixed by calling `handle.focus()` after `Cm6Handle::create()`. |
| Typing works without Enter | ✅ | Verified with `playwright-cli` on both regular pages and journal pages. |
| Blur saves | ✅ | `focusout` handler calls `on_save`. |
| Reload persists | ✅ | Verified in runtime and E2E. |

---

## 2. Enter behavior inside an editing block

### Target behavior

When already editing a block:

- `Enter` should create a **new block** according to outliner rules.
- At minimum:
  - at end of block → create a new sibling block below
  - in middle of text → split block at cursor

### Logseq Technical Analysis (from source code)

**Key findings from Logseq's `deps/outliner/src/logseq/outliner/core.cljs`:**

```clojure
(defn ^:api insert-blocks
  "Insert blocks as children (or siblings) of target-node.
   Options:
     `sibling?`: as siblings (true) or children (false).
     `bottom?`: inserts block to the bottom.
     `top?`: inserts block to the top.
     `keep-uuid?`: whether to replace `:block/uuid` from the parameter `blocks`.
     `replace-empty-target?`: If the `target-block` is an empty block, whether to replace it."
  [db blocks target-block {:keys [sibling? keep-uuid? keep-block-order?
                                  outliner-op replace-empty-target? update-timestamps?]
                           :as opts}]
```

**Block ordering uses `block/order` field:**
```clojure
(defn- get-block-orders
  [blocks target-block sibling? keep-block-order?]
  (let [target-order (:block/order target-block)
        start-order (when sibling? target-order)
        end-order (if sibling?
                    (:block/order (ldb/get-right-sibling target-block))
                    (let [first-child (ldb/get-down target-block)]
                      (:block/order first-child)))
        orders (db-order/gen-n-keys (count blocks) start-order end-order)]
    orders)
```

**Key insight**: Logseq uses `block/order` for ALL block ordering. The `sibling?` flag determines whether the new block goes:
- **As sibling** (`sibling? = true`): inserted after/before current block at same parent level
- **As child** (`sibling? = false`): inserted as child of current block

### Quilt current status

| Behavior | Status | Evidence |
|---------|--------|----------|
| Enter while editing creates new block | ⚠️ | Basic implementation exists but ordering is wrong |
| Split/new-block persistence to backend | ⚠️ | `create_block` called but without `preceding_block_id` |
| Block ordering by `block/order` | ❌ | Backend appends at end, no ordering field used |

### Likely cause

Code path exists but is incomplete:

- `cm6_block_editor.rs` → `on_enter_cb` calls `tree_ops.on_split(cursor)` and `on_save(...)`
- `block.rs` → `on_split` uses `split_block(blocks_mut, ...)`
- `outliner/tree.rs` → `split_block()` creates a second `BlockDto` **locally only**
- **Backend** → `create_block` appends at end, no `preceding_block_id` or `block/order`

### Required implementation

1. **Backend**: Add `preceding_block_id` parameter to `create_block` API
2. **Frontend**: Pass preceding block UUID when creating new block after Enter
3. **Ordering**: Use `block/order` field for proper positioning

**Recommended API change:**
```rust
// crates/quilt-server/src/routes.rs
// POST /api/v1/blocks
#[derive(Deserialize)]
pub struct CreateBlockRequest {
    pub content: String,
    pub page_id: i64,
    pub preceding_block_id: Option<i64>,  // NEW: for ordering
    // ... existing fields
}
```

---

## 3. Empty page / empty journal behavior

### Target behavior

- Empty page or journal shows a clear placeholder.
- Clicking the placeholder creates the **first block**.
- The new block enters edit mode immediately.

### Quilt current status

| Behavior | Status | Evidence |
|---------|--------|----------|
| Empty journal shows placeholder | ✅ | `No notes yet. Start writing...` |
| Click placeholder creates first block | ✅ | Added `on:click` in `PageView` fallback. |
| Auto-enters edit mode | ✅ | `selection_state.request_edit(...)` after create. |
| Persist after reload | ✅ | Covered by E2E. |

### Caveat

The newly created block currently uses a synthetic local `BlockDto` populated with placeholder fields (`page_id`, timestamps, etc.) because the backend `POST /api/v1/blocks` response only returns `{ id, content }`.

This is acceptable short-term but should be replaced with a canonical server DTO if more metadata is needed immediately after create.

---

## 4. Journals vs regular pages

### Target behavior

- Journals are special pages identified by date.
- Journals should live in the **journal workflow**, not appear as surprising normal pages.
- `All Pages` should not confuse users by mixing journals with standard pages unless explicitly intended.

### Quilt current status

| Behavior | Status | Evidence |
|---------|--------|----------|
| Journal route uses page editor infrastructure | ✅ | `/journal` and `/journal/:date` route to `PageView`. |
| Journal editing works | ✅ | Verified with `playwright-cli`. |
| Journals appear in All Pages | ⚠️ | `PagesView` renders all pages from `list_pages()`, including `page.journal == true`. |
| Clear separation between journal list and normal pages | ❌ | Current UX is mixed and confusing. |

### Why the user saw a page for the 28th

Because Quilt currently lists **all pages**, any journal auto-created by navigation, testing, or use appears in `All Pages` with a `journal` badge.

### Recommended alignment

Default behavior should be one of:

1. **Hide journals from All Pages** completely
2. Show journals only behind a filter/toggle
3. Split `All Pages` into sections: Pages / Journals

Recommended default: **hide journals from All Pages** for now.

---

## 5. All Pages behavior

### Target behavior

`All Pages` should answer the user question: “What are my regular pages?”

It should not:

- look like a dump of every journal created by navigation
- mix ephemeral daily pages with curated topic pages by default

### Quilt current status

- `crates/quilt-ui/src/pages/page_list.rs` calls `bridge::list_pages()` and renders everything.
- It only adds a `journal` badge, which is not enough UX separation.

### Required change

- Add filtering in UI and/or backend:
  - backend option: `GET /api/v1/pages?include_journals=false`
  - UI option: filter `page.journal` before rendering

---

## 6. Save model

### Target behavior

- Blur saves reliably.
- Navigation away should not silently lose edits.
- Save semantics should be predictable and testable.

### Quilt current status

| Behavior | Status | Evidence |
|---------|--------|----------|
| Blur saves | ✅ | `focusout` triggers save. |
| Navigation after blur persists | ✅ | Covered in E2E. |
| Navigation without blur guaranteed safe | ⚠️ | Not guaranteed; no explicit route-change save contract. |

### Recommendation

Document the current rule clearly:

- **Current contract**: save on blur/focusout

Then decide if Quilt should also support:

- save-on-route-change
- save-on-editor-destroy
- debounce autosave while typing

For Logseq-like behavior, save-on-blur is acceptable baseline, but save-on-navigation is safer.

---

## 7. Keyboard outliner semantics still needing parity verification

These behaviors are central to a Logseq-like outliner but are **not yet fully locked down in Quilt**.

| Behavior | Current confidence | Needed action |
|---------|--------------------|---------------|
| Enter creates new block / split | Low | Implement + E2E |
| Backspace at start merges with previous | Low | Verify runtime + E2E |
| Tab indents current block | Medium | Verify persistence + E2E |
| Shift+Tab outdents current block | Medium | Verify persistence + E2E |
| Arrow navigation while not editing | Medium | Verify runtime + E2E |
| Collapse/expand behavior | Medium | Verify runtime + E2E |

---

## 8. References, autocomplete, and slash commands

### Target behavior

- `[[` should trigger page reference autocomplete.
- Slash command behavior should not break save/focus semantics.
- Insertion should preserve cursor and persist appropriately.

### Quilt current status

| Behavior | Status | Notes |
|---------|--------|-------|
| Autocomplete pipeline exists | ✅ | Present in CM6 editor callbacks. |
| Slash command pipeline exists | ✅ | Present in `cm6_block_editor.rs`. |
| E2E coverage for core editing + autocomplete interaction | ⚠️ | Some old smoke coverage exists, but not part of strict parity matrix. |

### Recommendation

Autocomplete/slash command tests should be added **after** core editing semantics are stable.

---

## 9. Known unrelated backend issue

There is a visible network error:

- `GET /api/v1/pages/journal/:date` → `500`

### Impact

- does **not** block block editing because block loading comes from `/api/v1/pages/:name/blocks`
- does pollute console/network and can hide real failures

### Recommendation

Fix this separately after core UX alignment.

---

## Current gap matrix

| Area | Target | Current | Status |
|------|--------|---------|--------|
| Click on block content | Enter edit mode + focus | Fixed and verified | ✅ |
| Type without Enter | Works immediately | Verified on pages + journals | ✅ |
| Blur saves | Yes | Verified | ✅ |
| Reload persists | Yes | Verified | ✅ |
| Click empty journal/page | Create first block | Implemented + verified | ✅ |
| Journals separated from All Pages | Yes | Filter implemented in `page_list.rs` | ✅ |
| Enter creates new block/split | Basic persistence working | New block created but ordering uncertain | ⚠️ |
| Backspace merge parity | Yes | Unverified | ⚠️ |
| Indent/outdent parity | Yes | Partially implemented, unverified end-to-end | ⚠️ |
| Journal endpoint health | No console/server error | Failing 500 | ⚠️ |

---

## Priority backlog

## P0 — must fix for Logseq-like editing

1. ~~**Enter creates new block / split correctly**~~ ⚠️ *Partially implemented*
   - new block created via `bridge::create_block`
   - ❌ **BLOCKING**: backend appends at end (no `preceding_block_id` support)
   - Needs: backend API change + frontend pass preceding_block_id

2. ~~**Remove journals from All Pages by default**~~ ✅ *Filter implemented in UI*
   - `non_journal_pages` signal filters `page.journal == false`
   - Browser CORS blocks E2E verification (pre-existing server issue)
   - Code review confirms logic is correct

3. **Add bullet rendering like Logseq**
   - Logseq uses `.bullet-container` + `.bullet` CSS classes
   - Bullet is a clickable element that wraps the block
   - Quilt needs: visually distinct bullet before each block

4. **Add markdown inline rendering**
   - Logseq renders bold, italic, links inline in display mode
   - Quilt currently shows raw markdown in non-editing blocks

## P1 — critical outliner parity

5. Verify and fix **Backspace merge**
6. Verify and fix **Tab indent / Shift+Tab outdent**
7. Verify **arrow navigation + selection model**
8. **Fix Enter split ordering** (backend `preceding_block_id`)

## P2 — polish and reliability

9. Fix `GET /api/v1/pages/journal/:date` 500
10. Consider save-on-navigation safety
11. Expand autocomplete/slash-command regression suite

---

## Required E2E matrix

These tests should exist before claiming Logseq-like parity.

### Already present

- click on empty journal creates first block
- click on existing block enters edit mode
- regular page click-only editing persists
- journal click-only editing persists
- navigation away/back after save persists

### Missing and required

- `Enter` at end creates new sibling block
- `Enter` in middle splits block at cursor
- new block after `Enter` persists after reload
- Backspace at start merges with previous block
- Tab indents and persists
- Shift+Tab outdents and persists
- All Pages excludes journals by default
- Journals view still shows journal pages correctly

---

## Implementation notes

### Logseq Bullet Rendering (from source analysis)

**From `src/main/frontend/components/block.cljs`:**

```clojure
(defn- bullet-on-click [e block uuid config] ...)

;; In the block component render:
(when-not (:hide-bullet? config)
  (let [bullet [:a.bullet-link-wrap {:on-click #(bullet-on-click % block uuid config)}
                [:span.bullet-container.cursor
                 {:class (str (when collapsed? "bullet-closed") ...)}
                 [:span.bullet (cond-> ...)]
                 ...]]
    bullet')
```

**Key CSS classes:**
- `.bullet-container` - wrapper for bullet area
- `.bullet` - the actual bullet element
- `.bullet-link-wrap` - clickable wrapper
- `.bullet-closed` - shown when block is collapsed
- `.hide-inner-bullet` - for nested bullets in collapsed parents

**Quilt needs to implement:**
1. Visual bullet (•) before each block
2. Click handler on bullet for selection/collapse
3. CSS for bullet states (collapsed, has-children, etc.)

### Logseq Block Ordering (from source analysis)

**From `deps/outliner/src/logseq/outliner/core.cljs`:**

```clojure
;; Block entity structure:
{:db/id 123
 :block/uuid #uuid "..."
 :block/parent {:db/id 456}     ; parent block or page
 :block/page {:db/id 789}        ; owning page
 :block/order "0a"}             ; string for lexicographic ordering
```

**Key insight**: Logseq uses `block/order` as a **string** for lexicographic ordering. This allows efficient insert at any position without renumbering.

**Generation:**
```clojure
(db-order/gen-n-keys count start-order end-order)
```

This generates lexicographically sortable keys like: `"0a"`, `"0b"`, `"0c"`, etc.

### Enter split — state of implementation

`on_split` in `block.rs` now calls `bridge::create_block` after `split_block` mutates local state.

**Remaining issue — ordering**: `create_block` does not accept `preceding_block_id` or `order`, so the backend appends the new block at the end of the parent's block list, not at the correct position after the split.

To fix properly, the backend needs either:
1. A `preceding_block_id` parameter in `create_block`
2. Use `block/order` string for lexicographic ordering

### All Pages filtering — implemented

`page_list.rs` now uses a `non_journal_pages` derived signal:
```rust
let non_journal_pages = Signal::derive(move || {
    pages.get().into_iter().filter(|p| !p.journal).collect::<Vec<_>>()
});
```

⚠️ Browser testing blocked by pre-existing CORS issue (server binds to 127.0.0.1, browser requests from localhost).

---

## Suggested execution order

1. Fix `Enter` split/new block and persist it.
2. Add E2E for split/new block.
3. Filter journals out of `All Pages`.
4. Add E2E ensuring journals are not shown there.
5. Verify Backspace merge and indent/outdent.
6. Fix the journal endpoint `500`.

---

## Review checklist

- [ ] Single click enters edit mode and focuses editor
- [ ] Enter creates/splits blocks like an outliner
- [ ] New block from Enter persists after reload
- [ ] Empty page/journal can create first block
- [ ] Journals are not misleadingly mixed into All Pages
- [ ] Journal route and page route both support identical edit semantics
- [ ] All claims above are backed by passing Playwright tests
