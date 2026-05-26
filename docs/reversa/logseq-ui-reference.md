# Logseq UI/UX Reference — Exact Interaction Patterns

> Extracted from Logseq source code (ClojureScript), official docs, and observed behavior.
> Purpose: precise reference for Quilt's reimplementation.
> Date: 2026-05-24

---

## 1. Layout Structure

### 1.1 Overall Layout

```
┌────────────────────────────────────────────────────────────────────┐
│ #app-container                                                      │
│ ┌──────────┬─────────────────────────────┬───────────────────────┐ │
│ │ LEFT     │ MAIN CONTENT AREA           │ RIGHT SIDEBAR         │ │
│ │ SIDEBAR  │                             │                       │ │
│ │          │                             │                       │ │
│ │ ~250px   │ flex-1                      │ ~320px (resizable)    │ │
│ │ (resize) │                             │ up to 70% viewport    │ │
│ │ min:240  │                             │                       │ │
│ │ max:460  │                             │                       │ │
│ │          │                             │                       │ │
│ └──────────┴─────────────────────────────┴───────────────────────┘ │
│ ┌──────────────────────────────────────────────────────────────────┐│
│ │ BOTTOM BAR (optional, for mobile status)                         ││
│ └──────────────────────────────────────────────────────────────────┘│
└────────────────────────────────────────────────────────────────────┘
```

### 1.2 Left Sidebar

- **CSS ID**: `#left-sidebar`, class `cp__sidebar-left-layout`
- **Default width**: `--ls-left-sidebar-width` CSS custom property (default ~250px)
- **Min width**: 240px, **Max width**: 460px (enforced via `min/max offset` in drag handler)
- **Resizable**: Yes — via drag handle (`.left-sidebar-resizer`) using `interact.js`
  - Drag resizes `--ls-left-sidebar-width` on `document.documentElement.style`
  - Persisted to `localStorage` key `ls-left-sidebar-width`
- **Toggle**: Keyboard `t l`, hamburger button in header, or swipe on mobile
- **Mobile behavior**: Overlays as a drawer with shade mask (`shade-mask`)
  - Touch swipe right (>40px) to open, left (>30px) to close
  - CSS transform `translate3d` for smooth animation
  - `is-closing`, `is-open`, `is-touching` CSS classes control transitions

### 1.3 Main Content Area

- **Class**: `#main-content-container`
- **Takes remaining space**: `flex: 1` in the flex row
- **Scrolling**: Vertical scroll within the main container
- **Wide mode**: Toggle with `t w` — removes max-width constraint on content
- **Content max-widths** (Tailwind): `lsm: 600px`, `lmd: 728px`, `llg: 960px`

### 1.4 Right Sidebar

- **Class**: `cp__right-sidebar`
- **Default width**: ~320px, resizable up to 70% of viewport
- **Toggle**: Keyboard `t r` or button
- **Multiple items**: Can have multiple pages/blocks open as stacked panels
- **Close individual**: `c t` closes top item
- **Clear all**: `mod+c mod+c`
- **Open today in sidebar**: `mod+shift+j` (Mac) / `alt+shift+j` (others)
- **Each panel**: Contains a full page view or block embed

### 1.5 Header / Toolbar

- Top bar with: hamburger menu (left sidebar toggle), page title/breadcrumb, search button, right sidebar toggle
- Below header: breadcrumb navigation when zoomed into a block

---

## 2. Outliner Block Editing

### 2.1 Block Structure

Each block is an `<div>` with class `ls-block` containing:
- **Bullet/container**: A clickable circle bullet point
- **Content area**: Editable text (uses CodeMirror editor in DB version, contenteditable input in file version)
- **Children container**: Nested blocks indented below

### 2.2 Editing States

A block has two modes:
1. **Selected** (not editing): Block has blue left border highlight. Arrow keys navigate between blocks. `Enter` to start editing.
2. **Editing**: Block content is an active input/CodeMirror instance. Cursor is inside the text.

### 2.3 Inline Editing Keyboard Shortcuts

| Action | Key | Behavior |
|--------|-----|----------|
| **New sibling block** | `Enter` | Splits content at cursor position. Text before cursor stays in current block, text after cursor goes to new sibling below. If cursor at end, creates empty sibling. If block is collapsed, new block is sibling not child. |
| **New line in block** | `Shift+Enter` | Inserts a literal newline within the same block (soft break). |
| **Indent (make child)** | `Tab` | Moves current block to become the last child of its previous sibling. Preserves children. |
| **Outdent** | `Shift+Tab` | Moves current block up one level (becomes sibling of parent). Preserves children. |
| **Delete empty block** | `Backspace` | If block is empty AND cursor is at position 0, deletes the block. If previous block exists, content merges upward. Children are moved to the parent. |
| **Delete forward** | `Delete` | Forward delete within text. On empty block at end, no-op. |
| **Escape editing** | `Escape` (binding: `[]` empty) | Exits editing mode. Block becomes selected (not editing). Saves content. |
| **Auto-save** | On blur / navigation | Block is saved when focus leaves it, when navigating away, or on any structural operation. Uses `save-current-block!` which calls `save-block-if-changed!`. |

### 2.4 Block Content Splitting (Enter)

When pressing Enter at cursor position `pos` in text `value`:
```
fst-block-text = value[0:pos]
snd-block-text = value[pos:end].trimLeft()
```
- Current block gets `fst-block-text`
- New sibling gets `snd-block-text`
- If `fst-block-text` is blank and `snd-block-text` is not → insert BEFORE current block
- New block gets a fresh UUID via `db/new-block-id`

### 2.5 Block Navigation (Non-Editing)

| Action | Key |
|--------|-----|
| Move up | `Up` or `Ctrl+P` |
| Move down | `Down` or `Ctrl+N` |
| Move left | `Left` |
| Move right | `Right` |
| Select block up | `Alt+Up` |
| Select block down | `Alt+Down` |
| Select text up | `Shift+Up` |
| Select text down | `Shift+Down` |
| Open block for editing | `Enter` |
| Open selected blocks in sidebar | `Shift+Enter` |
| Select all blocks | `Mod+Shift+A` |
| Select parent | `Mod+A` |
| Delete selection | `Backspace` or `Delete` |

### 2.6 Block Structural Operations

| Action | Key | Platform |
|--------|-----|----------|
| Move block up | `Mod+Shift+Up` (Mac) / `Alt+Shift+Up` | Swaps with previous sibling |
| Move block down | `Mod+Shift+Down` (Mac) / `Alt+Shift+Down` | Swaps with next sibling |
| Move selected blocks | `Mod+Shift+M` | Opens search to pick destination |
| Zoom in (focus on block) | `Mod+.` (Mac) / `Alt+Right` | Shows block and descendants as full page |
| Zoom out (focus on parent) | `Mod+,` (Mac) / `Alt+Left` | Goes to parent block or page |
| Expand children | `Mod+Down` | Expands collapsed block |
| Collapse children | `Mod+Up` | Collapses block's children |
| Toggle collapse | `Mod+;` | Toggles collapse state |
| Toggle all open/closed | `t o` | Opens/closes all blocks globally |
| Toggle number list | `t n` | Toggles ordered list on selected blocks |

### 2.7 Text Formatting (In Editing Mode)

| Action | Key | Effect |
|--------|-----|--------|
| Bold | `Mod+B` | Wraps selection in `**...**` |
| Italic | `Mod+I` | Wraps selection in `*...*` |
| Highlight | `Mod+Shift+H` | Wraps selection in `^^...^^` |
| Strikethrough | `Mod+Shift+S` | Wraps selection in `~~...~~` |
| Insert link | `Mod+L` | Opens link input dialog |
| Clear block | `Ctrl+L` (Mac) / `Alt+L` | Clears all content in block |
| Kill line before cursor | `Ctrl+U` (Mac) / `Alt+U` | Deletes from start of block to cursor |
| Kill line after cursor | `Ctrl+K` (Mac-only default) / `Alt+K` | Deletes from cursor to end of block |
| Beginning of block | `Alt+A` (non-Mac) | Moves cursor to block start |
| End of block | `Alt+E` (non-Mac) | Moves cursor to block end |
| Forward word | `Ctrl+Shift+F` (Mac) / `Alt+F` | Moves cursor forward one word |
| Backward word | `Ctrl+Shift+B` (Mac) / `Alt+B` | Moves cursor backward one word |
| Forward kill word | `Ctrl+W` (Mac) / `Alt+D` | Deletes word forward |
| Backward kill word | `Alt+W` (non-Mac) | Deletes word backward |
| Copy block embed | `Mod+Shift+E` | Copies `{{embed ((block-uuid))}}` |
| Paste as single block | `Mod+Shift+V` | Pastes text into current block instead of creating new blocks |
| Insert YouTube timestamp | `Mod+Shift+Y` | Inserts YouTube timestamp macro |

### 2.8 Undo/Redo

| Action | Key |
|--------|-----|
| Undo | `Mod+Z` |
| Redo | `Mod+Shift+Z` or `Mod+Y` |

---

## 3. Block Hierarchy Visual

### 3.1 Bullet Points

- Each block has a bullet (circle) on the left side
- **Empty circle**: Block with no children (leaf node)
- **Filled/interactive circle**: Block has children — clickable to toggle collapse
- The bullet is rendered as a `<a>` tag with class `bullet` inside a `.block-control` container

### 3.2 Indentation

- Each nesting level adds left padding/margin (~24px per level, though exact value is theme-dependent)
- Indentation is rendered using nested `div` containers with class `block-children-container`
- The tree structure is: `.ls-block > .block-content-wrapper` and children go into `.block-children`

### 3.3 Collapse/Expand

- **Visual indicator**: A small toggle caret or rotated chevron on the bullet
- **Collapsed state**: Children are hidden. Bullet shows a different visual (often a right-pointing triangle or filled circle)
- **Child count**: When collapsed, no explicit count badge is shown by default (the bullet appearance changes to indicate "has children")
- **Collapse toggle**: Click bullet, or `Mod+Up`/`Mod+Down`, or `Mod+;` to toggle
- **Toggle all**: `t o` toggles all blocks open/closed
- **Behavior**: Collapsing a block only hides its children visually — the data remains. The collapsed state is stored in `block/collapsed?` property.

### 3.4 Block Selection

- Click a block bullet to select it (blue highlight)
- `Alt+Up`/`Alt+Down` to extend selection to multiple blocks
- Selected blocks show blue left border
- Selected blocks can be: deleted, moved, copied, indented/outdented together

---

## 4. Slash Commands

### 4.1 Trigger

Type `/` in an editing block to open the slash command dropdown.

### 4.2 Behavior

- Dropdown appears below the cursor position
- Fuzzy search filters commands as you type after `/`
- Navigate with `Up`/`Down` arrows (or `Ctrl+P`/`Ctrl+N`)
- `Enter` to execute, `Escape` to cancel
- The slash `/` is removed from text after selection

### 4.3 Complete Command List (DB Version)

Commands are grouped into categories:

#### Basic
| Command | Effect |
|---------|--------|
| **Node Reference** (`Page Reference`) | Inserts `[[]]` with page autocomplete, cursor between brackets |
| **Node Embed** (`Block Embed`) | Opens block search to embed a block reference |

#### Format
| Command | Effect |
|---------|--------|
| **Link** | Opens input dialog for URL + label, inserts markdown link |
| **Image Link** | Opens input dialog for image URL + label |
| **Code Block** | Creates a code block (exits editing, opens CodeMirror) |
| **Quote Block** | Sets block display type to `quote` |
| **Math Block** | Sets block display type to `math` (LaTeX) |
| **Underline** (Markdown mode only) | Inserts `<ins></ins>` tags |

#### Headings
| Command | Effect |
|---------|--------|
| **Normal Text** | Removes heading property |
| **Heading 1** through **Heading 6** | Sets block heading level 1-6 |

#### Task Status (DB version uses property-based statuses)
| Command | Effect |
|---------|--------|
| **TODO** | Sets status property to "Todo" |
| **DOING** (In Progress) | Sets status property to "Doing" |
| **DONE** | Sets status property to "Done" |
| **NOW** | Sets status property to "Now" |
| **LATER** | Sets status property to "Later" |
| **WAITING** | Sets status property to "Waiting" |
| **CANCELLED** | Sets status property to "Canceled" |

#### Task Date
| Command | Effect |
|---------|--------|
| **Deadline** | Opens date picker, sets Deadline property on block |
| **Scheduled** | Opens date picker, sets Scheduled property on block |

#### Priority
| Command | Effect |
|---------|--------|
| **No Priority** | Clears priority |
| **Priority A** | Sets priority property to "A" |
| **Priority B** | Sets priority property to "B" |
| **Priority C** | Sets priority property to "C" |

#### Time & Date
| Command | Effect |
|---------|--------|
| **Tomorrow** | Inserts `[[tomorrow's journal date]]` |
| **Yesterday** | Inserts `[[yesterday's journal date]]` |
| **Today** | Inserts `[[today's journal date]]` |
| **Current Time** | Inserts current time string (HH:mm) |
| **Date Picker** | Opens date picker to select arbitrary date |

#### List Type
| Command | Effect |
|---------|--------|
| **Ordered List (Own)** | Toggles numbered list on this block |
| **Ordered List (Children)** | Toggles numbered list on children |

#### Advanced
| Command | Effect |
|---------|--------|
| **Comment** | Opens comment input for block |
| **Query** | Inserts empty block and opens query builder |
| **Advanced Query** | Creates a block with query property (Clojure Datalog code block) |
| **Calculator** | Creates a code block with calc type |
| **Upload an asset** | Triggers file picker for image/file upload |
| **Template** | Opens template search to insert template |
| **Embed HTML** | Inserts `@@html: @@` inline HTML container |
| **Embed video URL** | Inserts `{{video URL}}` |
| **Embed YouTube timestamp** | Inserts YouTube timestamp macro |
| **Embed Twitter/X** | Inserts `{{tweet URL}}` |
| **Add property** | Opens new property editor on block |

#### Plugin Commands
- Dynamically added based on installed plugins
- Grouped under "Plugins" category

### 4.4 Cycle TODO

- `Mod+Enter` cycles through: `TODO → DOING → DONE → TODO`
- In DB version, this cycles the `logseq.property/status` property through `Todo → Doing → Done → Todo`

---

## 5. Wiki-Links (Page References)

### 5.1 Autocomplete Trigger

Type `[[` in a block to open the page autocomplete dropdown.

### 5.2 Autocomplete Behavior

- Dropdown shows matching pages filtered as you type
- **Fuzzy search**: Uses multi-extract fuzzy matching (page name, English name, pinyin for Chinese)
- **Navigation**: `Up`/`Down` arrows or `Ctrl+P`/`Ctrl+N`
- **Select**: `Enter` to select and insert `[[page-name]]`
- **Shift+Enter**: Selects and also opens in right sidebar
- **Mod+Enter**: Completes without closing bracket customization
- The `[[` prefix is replaced by `[[selected-page-name]]`
- Cursor ends after `]]`

### 5.3 Hashtag Pages

Type `#` followed by text to get page autocomplete for tags.
- `#tag-name` is equivalent to `[[tag-name]]` with a `#` prefix
- `#[[page name]]` syntax for multi-word tags

### 5.4 Page Reference Clicking

- **Left click** on `[[page-name]]`: Navigates to that page in main content area
- **Shift+Click**: Opens page in right sidebar
- **Ctrl/Cmd+Click**: Opens page (same as left click in practice)
- **Hover (desktop)**: Shows popup preview of page content after ~1 second delay
  - Preview shows page content in a 600px-wide popup
  - Disappears on mouse leave (300ms delay)
- **Non-existent page**: `[[New Page]]` renders with slightly dimmed brackets. Clicking creates the page.

### 5.5 Block References

- Type `((` to open block search autocomplete
- Inserts `((block-uuid))` which renders the referenced block's content inline
- Block references show the original content and update when source changes

---

## 6. Left Sidebar

### 6.1 Structure

```
┌──────────────────────┐
│ GRAPH SELECTOR       │  ← Dropdown to switch between graphs
│                      │
│ ─── NAVIGATIONS ──── │  ← Customizable section
│ 📅 Journals          │  ← Or custom home page
│ 📄 All Pages         │
│ 🔮 Graph View        │
│ 🃏 Flashcards        │
│ # Tasks              │  ← Optional, configurable
│ # Assets             │  ← Optional, configurable
│                      │
│ ─── FAVORITES ────── │  ← Drag-reorderable
│ ⭐ Page 1            │
│ ⭐ Page 2            │
│ ⭐ Page 3            │
│                      │
│ ─── RECENT ───────── │  ← Auto-populated
│ 📄 Recent Page 1     │
│ 📄 Recent Page 2     │
│ 📄 Recent Page 3     │
│                      │
├──────────────────────┤
│ resize handle ║      │
└──────────────────────┘
```

### 6.2 Graph Selector

- Dropdown at top of sidebar showing all open graphs
- Click to switch between graphs
- Can add/remove graphs
- Shortcut: `Alt+Shift+G` to open graph selector dialog

### 6.3 Favorites

- List of pinned pages, persisted in config
- **Drag-reorderable**: Uses DnD to reorder
- **Toggle favorite**: `Mod+Shift+F` on current page
- **Right-click / context menu**: Unfavorite, open in sidebar
- Click navigates to page; Shift+click opens in right sidebar
- Each favorite shows: page icon + page title

### 6.4 Recent Pages

- Automatically populated with recently visited pages
- Shows page icon + title
- Click to navigate; Shift+click to open in sidebar

### 6.5 Navigation Items

- Configurable: Flashcards, All Pages, Graph View, Tasks, Assets
- Users can toggle which items appear via filter edit button
- Stored in localStorage key `ls-sidebar-navigations`

---

## 7. Right Sidebar

### 7.1 Structure

```
┌─────────────────────────────┐
│ ┌─────────────────────────┐ │
│ │ Panel 1: Page/Block     │ │  ← Close button (×)
│ │                         │ │
│ │ (full page content or   │ │
│ │  block with children)   │ │
│ │                         │ │
│ ├─────────────────────────┤ │
│ │ Panel 2: Linked Refs    │ │  ← Close button (×)
│ │                         │ │
│ │ ...                     │ │
│ └─────────────────────────┘ │
│                              │
│ resize handle ║              │
└─────────────────────────────┘
```

### 7.2 Behavior

- **Multiple panels**: Each panel shows a page or block
- **Close top**: `c t` removes topmost panel
- **Clear all**: `Mod+C Mod+C` closes all panels and hides sidebar
- **Open in sidebar**:
  - Shift+Click any page link
  - `Mod+Shift+O` opens link under cursor in sidebar
  - Right-click → "Open in sidebar"
- **Panel content**: Full page view with linked references at bottom
- **Linked References**: Shows blocks that reference this page via `[[page-name]]`
- **Unlinked References**: Shows blocks that mention the page name without explicit link
- **Page Graph**: Small graph visualization showing connections (optional)

### 7.3 Page Properties (Bottom of Page)

- Below the last block, shows page properties in a key-value table
- Properties: tags, aliases, icon, custom properties
- Editable inline

---

## 8. Journal System

### 8.1 Journal Pages

- Each day gets a page named with the date (e.g., "May 24, 2026")
- Journal page title format is locale-dependent
- Journal pages are created on demand (when navigated to)
- The home page defaults to today's journal (configurable)

### 8.2 Journal Navigation

| Shortcut | Action |
|----------|--------|
| `g j` | Go to journals (home) |
| `g t` | Go to tomorrow's journal |
| `g n` | Go to next journal |
| `g p` | Go to previous journal |

### 8.3 Calendar Widget

- In the left sidebar, below the journals navigation
- Monthly calendar grid showing dates
- Click a date to navigate to that journal page
- Current date is highlighted
- Previous/next month navigation

### 8.4 Journal Page Content

- Starts empty (or with a template if configured)
- Blocks are added just like any other page
- Journal pages serve as the daily "inbox" for notes
- All journal pages can be listed at `/all-journals` route

---

## 9. Search (Ctrl+K)

### 9.1 Search Modal

- **Trigger**: `Mod+K` for global search, `Mod+Shift+K` for in-page search
- **Appearance**: Modal overlay (not sidebar), dark background overlay
- **Input**: Single text input at top, auto-focused
- **Results**: Paginated list below input, grouped by type

### 9.2 Search Modes

| Mode | Trigger | Description |
|------|---------|-------------|
| Global search | `Mod+K` | Searches across all pages and blocks |
| Page search | `Mod+Shift+K` | Searches within current page |
| Command palette | `Mod+Shift+P` | Searches commands (keyboard shortcuts) |
| Themes | `Mod+Shift+I` (Mac) / `Alt+Shift+I` | Searches themes |
| Find in page (Electron) | `Mod+F` | Browser-native find |

### 9.3 Search Behavior

- **Fuzzy search**: Matches partial strings, not just exact matches
- **Result types**: Pages, blocks
- **Navigation**: Arrow keys to navigate results, `Enter` to open
- **Re-index**: `Mod+C Mod+S` rebuilds the search index
- **Block search**: Shows block content preview with match highlighted

### 9.4 Auto-Complete in Editor

When typing `[[`, `((`, `#`, or `/` in editing mode:
- Inline dropdown appears below cursor
- Fuzzy filtered as you type
- Arrow keys to navigate, Enter to select
- Escape to dismiss

---

## 10. Complete Keyboard Shortcuts Reference

### 10.1 Shortcut Handler Architecture

Logseq uses Google Closure's `KeyboardShortcutHandler` with multiple handler groups:
- `:shortcut.handler/block-editing-only` — Active only when editing a block
- `:shortcut.handler/editor-global` — Active when not in component editing
- `:shortcut.handler/global-prevent-default` — Always active, prevents browser defaults
- `:shortcut.handler/global-non-editing-only` — Active only when NOT editing
- `:shortcut.handler/misc` — Always active (handles copy for chord detection)

### 10.2 Complete Shortcut Table

**Note**: `Mod` = `Cmd` on macOS, `Ctrl` on others. Bindings marked `(Mac)` differ on macOS.

#### Basics
| Shortcut | Action |
|----------|--------|
| `Mod+K` | Global search |
| `Mod+Shift+P` | Command palette |
| `Mod+Shift+K` | Search in current page |
| `Mod+Z` | Undo |
| `Mod+Shift+Z` / `Mod+Y` | Redo |
| `Mod+F` (Electron) | Find in page |
| `Mod+S` (DB version) | Save graph |

#### Navigation
| Shortcut | Action |
|----------|--------|
| `g h` | Go home (journals) |
| `g j` | Go to journals |
| `g a` | Go to all pages |
| `g g` | Go to graph view |
| `g Shift+G` | Go to all graphs |
| `g s` | Go to keyboard shortcuts |
| `g t` | Go to tomorrow |
| `g n` | Go to next journal |
| `g p` | Go to previous journal |
| `g f` / `t c` | Toggle flashcards |
| `Mod+[` | Go back (browser history) |
| `Mod+]` | Go forward (browser history) |
| `Mod+J` | Jump to (block/page) |

#### Block Editing
| Shortcut | Action |
|----------|--------|
| `Enter` | New sibling block / Open selected block |
| `Shift+Enter` | New line in block / Open in sidebar (selected) |
| `Tab` | Indent (make child) |
| `Shift+Tab` | Outdent |
| `Backspace` | Delete backward / Delete empty block |
| `Delete` | Delete forward |
| `Mod+Enter` | Cycle TODO status |
| `Mod+Up` | Collapse block children |
| `Mod+Down` | Expand block children |
| `Mod+;` | Toggle collapse |
| `Mod+.` (Mac) / `Alt+Right` | Zoom in |
| `Mod+,` (Mac) / `Alt+Left` | Zoom out |
| `Mod+Shift+Up` (Mac) / `Alt+Shift+Up` | Move block up |
| `Mod+Shift+Down` (Mac) / `Alt+Shift+Down` | Move block down |
| `Mod+Shift+M` | Move selected blocks |
| `Mod+O` | Follow link under cursor |
| `Mod+Shift+O` | Open link in sidebar |
| `Alt+Up` | Select block up |
| `Alt+Down` | Select block down |
| `Mod+A` | Select parent block |
| `Mod+Shift+A` | Select all blocks |

#### Formatting
| Shortcut | Action |
|----------|--------|
| `Mod+B` | Bold |
| `Mod+I` | Italic |
| `Mod+Shift+H` | Highlight |
| `Mod+Shift+S` | Strikethrough |
| `Mod+L` | Insert link |
| `Mod+E` (Mac) / `Mod+Alt+E` | Quick add (new block anywhere) |

#### Properties (Selection Mode)
| Shortcut | Action |
|----------|--------|
| `Mod+P` (Mac) / `Ctrl+Alt+P` | Add property |
| `p t` | Set Tags property |
| `p d` | Set Deadline property |
| `p s` | Set Status property |
| `p p` | Set Priority property |
| `p i` | Set Icon property |
| `p r` | Add reaction |
| `p a` | Toggle display hidden properties |
| `Ctrl+Space` | Add comment |

#### Toggle UI
| Shortcut | Action |
|----------|--------|
| `t b` | Toggle brackets visibility |
| `t d` | Toggle document mode |
| `t l` | Toggle left sidebar |
| `t r` | Toggle right sidebar |
| `t w` | Toggle wide mode |
| `t t` | Toggle theme (dark/light) |
| `t o` | Toggle open/close all blocks |
| `t n` | Toggle number list |
| `t s` | Toggle settings |
| `t i` | Select theme color |
| `t p` | Go to plugins (if enabled) |
| `Shift+/` | Toggle help |
| `c c` | Customize appearance |

#### Sidebar
| Shortcut | Action |
|----------|--------|
| `Mod+Shift+J` (Mac) / `Alt+Shift+J` | Open today in sidebar |
| `c t` | Close top sidebar panel |
| `Mod+C Mod+C` | Clear sidebar |

#### Copy/Paste
| Shortcut | Action |
|----------|--------|
| `Mod+C` | Copy (blocks or text) |
| `Mod+Shift+C` | Copy as text |
| `Mod+X` | Cut |
| `Mod+Shift+E` | Copy block embed |
| `Mod+Shift+V` | Paste text in one block |

#### Graph
| Shortcut | Action |
|----------|--------|
| `Alt+Shift+G` | Open graph selector |
| `Mod+C Mod+S` | Re-index search |

#### Window (Electron only)
| Shortcut | Action |
|----------|--------|
| `Mod+W` | Close window |
| `Mod+M` | Publish dialog |
| `Mod+Shift+1` | Run shell command |

---

## 11. Theme System

### 11.1 Theme Architecture

Logseq uses a CSS custom property based theming system combined with Tailwind CSS and Radix UI colors.

#### Core CSS Custom Properties

```css
:root {
  /* Primary theme colors */
  --ls-primary-background-color
  --ls-secondary-background-color
  --ls-tertiary-background-color
  --ls-quaternary-background-color

  /* Left sidebar */
  --ls-left-sidebar-width  /* default ~250px */

  /* Accent (Shui/Radix integration) */
  --accent
  --accent-foreground

  /* Radix-compatible gray scale */
  --lx-gray-01 through --lx-gray-12
  --lx-gray-01-alpha through --lx-gray-12-alpha

  /* Accent scale */
  --lx-accent-01 through --lx-accent-12
  --lx-accent-01-alpha through --lx-accent-12-alpha
}
```

### 11.2 Theme Toggling

- **Default**: Dark theme
- **Toggle**: `t t` switches between dark and light
- **Custom themes**: CSS files in `~/.logseq/graph-name/custom.css` or loaded via plugins
- **Theme marketplace**: `t i` opens theme color selector
- **Tailwind dark mode**: Uses `darkMode: 'class'` — toggles `dark` class on HTML element

### 11.3 Color System (Tailwind Config)

- **Radix colors** mapped to Tailwind scale: red, pink, orange, yellow, green, blue, indigo, purple, plus amber, bronze, brown, crimson, cyan, gold, grass, lime, mauve, mint, olive, plum, sage, sand, sky, slate, teal, tomato, violet
- **Custom accent color**: `--accent` HSL variable, with 12 opacity steps
- **Custom gray**: Mapped through `--lx-gray-*` variables for theme independence
- **Border radius**: Uses `--radius` CSS variable (Shadcn-style)

---

## 12. Drag and Drop

### 12.1 Block Drag and Drop

- **Library**: Uses `interact.js` for drag interactions
- **Draggable elements**: Blocks (`.ls-block`), sidebar favorites, images
- **Visual feedback**:
  - `*dragging?` atom tracks drag state
  - `*dragging-block` stores the block being dragged
  - `*dragging-over-block` stores the drop target
  - CSS classes added during drag: `is-resizing`, `is-resizing-buf`

### 12.2 Block DnD Behavior

- **Drag a block**: Grabs the block and all its children
- **Drop targets**:
  - **Between blocks**: Insert as sibling at that position
  - **On a block's bullet**: Make it a child of that block (last child)
- **Drop indicator**: Visual line/gap showing where the block will land
- **Move modes**: `*move-to` atom controls whether it's a sibling or child insertion

### 12.3 Image Resizing

- Images have left and right resize handles (`.handle-left`, `.handle-right`)
- Dragging handles resizes image width
- Width stored as `logseq.property.asset/width` property on the block
- Uses `interact.js` for drag

### 12.4 Sidebar Favorites Reorder

- Favorite items in left sidebar are drag-reorderable
- Uses DnD component (`dnd-component/items`)
- On drag end, calls `page-handler/<reorder-favorites!`
- Persisted in graph config

### 12.5 Left Sidebar Resize

- `.left-sidebar-resizer` handle on right edge
- Drag to resize between 240px and 460px
- Width persisted to localStorage

---

## 13. Additional UI Patterns

### 13.1 Command Palette

- **Trigger**: `Mod+Shift+P`
- Shows all available commands with keyboard shortcuts
- Search/filter commands
- Execute by pressing Enter

### 13.2 Date Picker

- Appears inline in block when setting Deadline/Scheduled
- Calendar grid to pick a date
- Can also type date directly

### 13.3 Notifications

- Toast-style notifications in bottom-right
- Types: success, warning, error
- Auto-dismiss
- Clear all: internal shortcut (not user-facing by default)

### 13.4 Context Menus

- Right-click on blocks: Cut, Copy, Copy block reference, Open in sidebar, Add property, etc.
- Right-click on page references: Open, Open in sidebar, Unfavorite
- Implemented using Shadcn popup/dropdown-menu

### 13.5 Block Hover Preview

- Hovering over a block reference `((uuid))` shows preview popup
- Hovering over a page reference `[[name]]` shows preview popup after 1s delay
- Preview shows content in a 600px-wide popup
- Disappears 300ms after mouse leaves

### 13.6 Mobile Touch Gestures

- **Swipe right** (>40px) on left edge: Open left sidebar
- **Swipe left** (>30px) on sidebar: Close left sidebar
- Uses `on-touch-start/move/end` handlers with `translate3d` transforms
- `is-touching` class during active swipe for CSS transitions

### 13.7 Document Mode

- **Toggle**: `t d`
- Shows content without bullet points — flat document view
- Blocks still editable but appear as paragraphs

### 13.8 Bracket Visibility

- **Toggle**: `t b`
- Shows/hides the `[[` `]]` brackets around page references
- When hidden, page references appear as plain colored text

---

## 14. Key Implementation Details for Quilt

### 14.1 Editor Implementation

- Logseq uses CodeMirror as the block editor in the DB version
- Each block becomes an editor instance when focused
- The editor handles: text input, cursor management, selection
- Structural operations (indent, new block, etc.) are handled outside the editor

### 14.2 State Management

- Central atom `state/state` holds all app state
- Reactive subscriptions via `rum.core/reactive` mixin
- Events published via `state/pub-event!` and consumed by handlers
- Key state keys:
  - `:editor/edit-block` — Currently editing block
  - `:editor/edit-input-id` — DOM ID of the editor input
  - `:editor/latest-shortcut` — Last shortcut triggered
  - `:editor/pending-new-block` — State for block creation animation
  - `:selection/blocks` — Set of selected block IDs
  - `:ui/left-sidebar-open?` — Left sidebar state
  - `:pdf/current` — Current PDF being viewed

### 14.3 Outliner Operations

All block operations go through `ui-outliner-tx/transact!` which wraps:
- `outliner-op/insert-blocks!` — Insert new blocks
- `outliner-op/delete-blocks!` — Delete blocks
- `outliner-op/move-blocks!` — Move/reorder blocks
- `outliner-op/save-block!` — Save block content changes

These operations maintain:
- Parent-child relationships (`:block/parent`, `:block/_parent`)
- Left sibling ordering (`:block/left`)
- Page membership (`:block/page`)
- Block order within parent

### 14.4 Block Data Model (DB Version)

```clojure
{:block/uuid       #uuid "..."
 :block/title      "Block content text"
 :block/page       {:db/id ...}
 :block/parent     {:db/id ...}
 :block/left       {:db/id ...}
 :block/format     :markdown   ; or :org
 :block/collapsed? false
 :block/heading-level nil      ; 1-6 or nil
 ;; Properties (DB version)
 :logseq.property/status     {:db/id ...}   ; Todo, Doing, Done, etc.
 :logseq.property/priority   {:db/id ...}   ; A, B, C
 :logseq.property/scheduled  "2026-05-24"
 :logseq.property/deadline   "2026-05-30"
 :logseq.property/tags       #{...}
 :block/tags                 #{...}}
```
