# Quilt — Behavior Specification (Prescriptive)

> Derived from `logseq-ui-reference.md` (descriptive) and `outliner-professional-baseline.md` (product decisions).
> **This document is PRESCRIPTIVE**: it defines how Quilt MUST behave, not how Logseq happens to behave.
> **Status**: Baseline spec. Not all features are implemented yet.

---

## 1. Block Editing

### 1.1 Enter — Split Block at Cursor

**WHEN** user presses `Enter` inside a block with cursor at position `pos` in text `value`
**THEN**:
- `fst_text = value[0:pos]`
- `snd_text = value[pos:end].trimStart()`
- Current block gets `fst_text`
- New sibling block gets `snd_text`
- New block gets fresh UUID
- New block inherits format (Markdown), level, and page_id from current
- IF `fst_text` is empty AND `snd_text` is not empty → new block is inserted BEFORE current
- IF block is collapsed → new block is sibling, NOT child
- IF cursor is at end (pos == len) → creates empty sibling
- Operation is recorded in HistoryStack as `OutlinerCommand::Split`

### 1.2 Shift+Enter — Soft Break

**WHEN** user presses `Shift+Enter` inside a block
**THEN**:
- A literal `\n` is inserted at cursor position
- Block is NOT split
- This is a text operation, NOT a structural operation

### 1.3 Tab — Indent (Make Child)

**WHEN** user presses `Tab` on a block
**THEN**:
- Block becomes child of its previous sibling
- Block preserves its children
- Block becomes last child of new parent
- IF block is first sibling (no previous sibling) → Tab is no-op
- Operation: `OutlinerCommand::Indent`

### 1.4 Shift+Tab — Outdent

**WHEN** user presses `Shift+Tab` on a block
**THEN**:
- Block moves up one level (becomes sibling of parent)
- Block preserves its children
- IF block is already at top level → Shift+Tab is no-op
- Operation: `OutlinerCommand::Outdent`

### 1.5 Backspace on Empty Block

**WHEN** block is empty AND cursor is at position 0 AND user presses `Backspace`
**THEN**:
- Block is deleted
- IF previous block exists → content merges upward (merge_with_previous)
- Children are moved to the parent block
- Operation: `OutlinerCommand::DeleteBlock`

### 1.6 Escape — Exit Editing

**WHEN** user presses `Escape` while editing a block
**THEN**:
- Block saves current content
- Block exits editing mode
- Block becomes selected (not editing)
- Focus stays on the block (selected mode)

---

## 2. Block Navigation (Selected Mode)

### 2.1 Arrow Keys

**WHEN** block is selected (not editing)
**THEN**:
- `Up`: focus moves to previous sibling or parent
- `Down`: focus moves to next sibling or first child
- `Left`: collapse block OR move to parent block
- `Right`: expand block OR move to first child
- `Enter`: start editing the selected block

### 2.2 Multi-Block Selection

**WHEN** user holds `Alt+Up` or `Alt+Down`
**THEN**:
- Selection expands/contracts to include adjacent blocks
- Selected blocks show blue left border
- Selected blocks can be: deleted (`Backspace`/`Delete`), indented (`Tab`), outdented (`Shift+Tab`), moved

### 2.3 Select Parent and Select All

**WHEN** user presses `Mod+A`
**THEN**: parent block of current selection is selected

**WHEN** user presses `Mod+Shift+A`
**THEN**: all blocks on current page are selected

---

## 3. Structural Operations

### 3.1 Move Block Up/Down

**WHEN** user presses `Mod+Shift+Up` (Mac) or `Alt+Shift+Up`
**THEN**: block swaps position with previous sibling

**WHEN** user presses `Mod+Shift+Down` (Mac) or `Alt+Shift+Down`
**THEN**: block swaps position with next sibling

### 3.2 Zoom In/Out

**WHEN** user presses `Mod+.` (Mac) / `Alt+Right`
**THEN**: selected block and its descendants become the full page view (breadcrumb shows path)

**WHEN** user presses `Mod+,` (Mac) / `Alt+Left`
**THEN**: zoom out to parent block or page

### 3.3 Collapse/Expand

**WHEN** user presses `Mod+;`
**THEN**: toggle collapse/expand of selected block's children

**WHEN** user presses `Mod+Up`
**THEN**: collapse selected block's children

**WHEN** user presses `Mod+Down`
**THEN**: expand selected block's children

---

## 4. Collapse/Expand Visual

### 4.1 Bullet Points

- **Empty circle** (○): block with no children
- **Filled circle** (●): block has children — clickable to toggle collapse
- **Collapsed indicator**: filled circle with a different visual (chevron or style)
- Click on bullet = toggle collapse

### 4.2 Child Count

- No explicit count badge is shown
- The bullet appearance alone indicates "has children"
- Collapsed children are hidden but data is preserved
- Collapsed state is stored in `block.collapsed` (boolean)

---

## 5. Page References (`[[Page]]`)

### 5.1 Autocomplete

**WHEN** user types `[[` in editing mode
**THEN**:
- Autocomplete dropdown appears below cursor
- Shows pages matching typed text (fuzzy search, case-insensitive)
- Navigation: `Up`/`Down` or `Ctrl+P`/`Ctrl+N`
- `Enter`: inserts `[[selected-page-name]]`, cursor after `]]`
- `Shift+Enter`: selects and opens page in right sidebar
- `Escape`: dismisses dropdown, `[[` remains
- The `[[` prefix is replaced by `[[selected-page-name]]`

### 5.2 Clicking a Page Reference

- **Left click**: navigate to that page in main content area
- **Shift+Click**: open page in right sidebar
- **Mod+Click**: same as left click

### 5.3 Non-Existent Page

- `[[New Page]]` renders with slightly dimmed brackets
- Clicking creates the page (navigates to blank page with that name)

### 5.4 Hover Preview

- **Desktop**: hovering over `[[Page]]` shows popup preview after ~1 second delay
- Preview shows first few blocks of the page
- Popup is ~600px wide
- Disappears 300ms after mouse leaves

---

## 6. Block References (`((Block))`)

### 6.1 Autocomplete

**WHEN** user types `((` in editing mode
**THEN**:
- Block search autocomplete opens
- Shows blocks matching typed text (fuzzy search)
- Navigation: `Up`/`Down`
- `Enter`: inserts `((block-uuid))`
- `Escape`: dismisses

### 6.2 Rendered Block Reference

- `((block-uuid))` renders the referenced block's content inline
- Updates when source block changes
- Shows original content and formatting

### 6.3 Hover Preview

- Hovering over `((uuid))` shows preview popup with block's content + children

---

## 7. Formatting (Inline)

### 7.1 Bold `**text**`

- `Mod+B` wraps selection or toggles bold at cursor
- Rendered as bold text

### 7.2 Italic `*text*`

- `Mod+I` wraps selection or toggles italic at cursor
- Rendered as italic text

### 7.3 Highlight `^^text^^`

- `Mod+Shift+H` wraps selection or toggles highlight at cursor
- Rendered with background highlight color
- EXTENSIBLE: `^^[#ff0]text^^` for colored highlights (future)

### 7.4 Strikethrough `~~text~~`

- `Mod+Shift+S` wraps selection or toggles strikethrough
- Rendered with strikethrough line

### 7.5 Code Inline `` `text` ``

- `` Mod+` `` wraps selection or toggles inline code
- Rendered in monospace with background

### 7.6 Link `[label](url)`

- `Mod+L` opens link input dialog
- Inserts `[label](URL)` or wraps selection as `[selection](URL)`

---

## 8. Undo/Redo

### 8.1 Unified History

- EVERY operation that changes structure, content, or properties goes through `HistoryStack`
- `Mod+Z` = undo last operation
- `Mod+Shift+Z` or `Mod+Y` = redo
- Text content changes are OUTLINER commands, not editor commands
- CodeMirror's own history extension is DISABLED

### 8.2 Reversible Operations

All these must be undoable:
- Text content change
- Split / Merge blocks
- Indent / Outdent
- Move block up/down
- Create / Delete block
- Change property (status, priority, deadline, scheduled, tags)
- Add / Remove ref
- Cycle status

---

## 9. Slash Commands

### 9.1 Trigger

- Type `/` in an editing block → slash command dropdown

### 9.2 Behavior

- Dropdown appears below cursor
- Fuzzy search filters as you type after `/`
- `Up`/`Down` navigate, `Enter` executes, `Escape` cancels
- The `/` is removed from text after selection

### 9.3 MUST Commands (v1)

| Command | Action |
|---------|--------|
| TODO | Set status property to `TODO` |
| DOING | Set status property to `DOING` |
| DONE | Set status property to `DONE` |
| Priority A/B/C | Set priority property |
| Deadline | Open date picker, set deadline |
| Scheduled | Open date picker, set scheduled |
| Today | Insert `[[today's journal date]]` |
| Tomorrow | Insert `[[tomorrow's journal date]]` |
| Page Reference | Insert `[[ ]]` with autocomplete |
| Block Embed | Open block search to embed |
| Template | Open template search |
| Code Block | Insert code block |

---

## 10. Right Sidebar

### 10.1 Structure

- Default width: ~320px, resizable
- Toggle: `t r` or button
- Multiple panels stacked vertically

### 10.2 Backlinks

- Shows blocks that reference the current page via `[[page-name]]` or `RefIndex::get_backlinks()`
- Backlinks are O(1) from in-memory RefIndex
- Each backlink shows: source page, block content, context

### 10.3 Unlinked References (v2)

- Shows blocks that mention the page name WITHOUT explicit `[[ ]]`
- Computed via FTS5 query
- Shows match with highlighted context

### 10.4 Panel Operations

- `Shift+Click` page ref → open in sidebar
- `Mod+Shift+O` → open link under cursor in sidebar
- `c t` → close top panel
- `Mod+C Mod+C` → clear all panels

---

## 11. Left Sidebar

### 11.1 Structure

```
Graph Selector
─────────────────
📅 Journals / Home
📄 All Pages
─────────────────
⭐ Favorites (drag-reorderable)
─────────────────
🕐 Recent Pages (auto)
```

### 11.2 Favorites

- Toggle favorite: `Mod+Shift+F` on current page
- Drag-reorderable via DnD
- Click → navigate; Shift+Click → open in right sidebar

### 11.3 Recent Pages

- Auto-populated with recently visited pages
- Click → navigate; Shift+Click → open in sidebar

---

## 12. Journals

### 12.1 Journal Pages

- Each day gets a page named with date (e.g., "May 26, 2026")
- Created automatically when navigated to
- Home page defaults to today's journal

### 12.2 Navigation

| Shortcut | Action |
|----------|--------|
| `g j` | Go to today's journal |
| `g t` | Go to tomorrow |
| `g n` | Go to next journal |
| `g p` | Go to previous journal |

### 12.3 Calendar

- Monthly calendar grid in left sidebar
- Click date → navigate to that journal page
- Current date highlighted
- Previous/next month navigation

---

## 13. Drag & Drop

### 13.1 Block DnD

- Drag a block + all its children
- **Drop between blocks**: insert as sibling at that position
- **Drop on a block's bullet**: make it a child (last child)
- Visual drop indicator (line/gap showing landing position)
- Operation goes through HistoryStack (undoable)

### 13.2 Sidebar Favorites

- Drag-reorderable in left sidebar
- Persisted in graph config

---

## 14. Theme System

- CSS custom property based (inherited from current Quilt theme system)
- Dark theme default
- `t t` toggle dark/light
- Accent colors defined via `--accent` variables
- Tailwind `dark:` class toggled on HTML element

---

## 15. Properties Inline

### 15.1 Visual

- Properties render inline with visual distinction (not plain text)
- `status:: TODO` renders with marker circle + text
- `priority:: A` renders with badge
- `scheduled:: 2026-05-26` renders with calendar icon
- `deadline:: 2026-05-30` renders with deadline icon
- `tags:: a, b, c` renders as tag pills
- `template:: name` renders with template icon

### 15.2 Editing

- Click on property value → edit inline or open dropdown
- `status::` → dropdown with TODO/DOING/DONE options
- `priority::` → dropdown with A/B/C
- `scheduled::` / `deadline::` → date picker
- `tags::` → tag input with autocomplete

### 15.3 Validation

- `status:: invalid` → error highlight (only TODO/DOING/DONE accepted)
- `priority:: X` → error (only A/B/C accepted)
- `scheduled:: not a date` → error
