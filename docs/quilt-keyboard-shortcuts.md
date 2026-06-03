# Quilt — Canonical Keyboard Shortcuts

> Extracted from Logseq behavior (`logseq-ui-reference.md` §10) and adapted for Quilt.
> **Canonical**: this is the source of truth for what keystrokes Quilt MUST implement.
> **Status**: Spec — not all shortcuts are implemented yet. See Roadmap Phase 4.

**Note**: `Mod` = `Cmd` on macOS, `Ctrl` on Linux/Windows.

---

## 1. Shortcut Handler Architecture

### 1.1 Modes

Quilt has four input handler groups (mirroring Logseq's architecture):

| Group | Active when | Examples |
|-------|-------------|----------|
| `block-editing-only` | Editing text inside a block | `Enter`, `Tab`, `Mod+B` |
| `block-selected-only` | Block selected (not editing) | `Enter` (start editing), `Backspace` (delete) |
| `editor-global` | Any mode except component editing | `Mod+Z`, `Mod+K` |
| `global-non-editing-only` | NOT editing | `g j`, `t l` |

### 1.2 Dispatcher

```rust
// InputHandler::dispatch(event, mode)
fn dispatch(key: Key, mods: Modifiers, mode: InputMode) -> Option<OutlinerIntent>

// Mode determines which handler group is active
enum InputMode {
    Editing,       // block-editing-only
    Selected,      // block-selected-only
    Global,        // editor-global
    Navigation,    // global-non-editing-only
    Autocomplete,  // autocomplete dropdown active
}
```

---

## 2. Complete Shortcut Table

### 2.1 Global Shortcuts (always active or global)

| Shortcut | Action | Intent | Implemented |
|----------|--------|--------|-------------|
| `Mod+K` | Global search | `Search::Global` | ❌ |
| `Mod+Shift+P` | Command palette | `Search::Commands` | ❌ |
| `Mod+Shift+K` | Search in current page | `Search::Page` | ❌ |
| `Mod+Z` | Undo | `History::Undo` | ✅ |
| `Mod+Shift+Z` / `Mod+Y` | Redo | `History::Redo` | ✅ |
| `Mod+S` | Save graph | `Graph::Save` | ❌ |

### 2.2 Block Editing (inside focused block)

| Shortcut | Action | Intent | Implemented |
|----------|--------|--------|-------------|
| `Enter` | New sibling block (split at cursor) | `EnterPressed { cursor }` | ✅ |
| `Shift+Enter` | New line in block (soft break) | `SoftBreak` | ❌ |
| `Tab` | Indent (make child) | `TabPressed` | ✅ |
| `Shift+Tab` | Outdent | `ShiftTabPressed` | ✅ |
| `Backspace` (empty, cursor=0) | Delete empty block, merge up | `BackspaceOnEmpty` | ⚠️ |
| `Delete` | Forward delete | `DeleteForward` | ❌ |
| `Escape` | Exit editing mode | `EscapeEditing` | ❌ |
| `Mod+Enter` | Cycle TODO status | `CycleStatus` | ⚠️ Partial |
| `Mod+Up` | Collapse block children | `Collapse` | ❌ |
| `Mod+Down` | Expand block children | `Expand` | ❌ |
| `Mod+;` | Toggle collapse | `ToggleCollapse` | ❌ |
| `Mod+.` (Mac) / `Alt+Right` | Zoom in (focus on block) | `ZoomIn` | ❌ |
| `Mod+,` (Mac) / `Alt+Left` | Zoom out (go to parent) | `ZoomOut` | ❌ |
| `Mod+Shift+Up` / `Alt+Shift+Up` | Move block up | `MoveUp` | ❌ |
| `Mod+Shift+Down` / `Alt+Shift+Down` | Move block down | `MoveDown` | ❌ |
| `Mod+Shift+M` | Move selected blocks | `MoveSelected` | ❌ |
| `Mod+Enter` | Follow link under cursor (nearest `[[Page]]`, `((block))`, `#tag`, URL) | `FollowLink` | ✅ (Quilt) |
| `Mod+Shift+Enter` | Open link in sidebar | `OpenInSidebar` | ❌ |
| `Mod+O` | Alternative follow link (Logseq alias, unconfirmed) | `FollowLink` | ❌ |

### 2.3 Text Formatting (inside editing block)

| Shortcut | Action | Markdown | Intent | Implemented |
|----------|--------|----------|--------|-------------|
| `Mod+B` | Bold | `**text**` | `FormatBold` | ❌ |
| `Mod+I` | Italic | `*text*` | `FormatItalic` | ❌ |
| `Mod+Shift+H` | Highlight | `^^text^^` | `FormatHighlight` | ❌ |
| `Mod+Shift+S` | Strikethrough | `~~text~~` | `FormatStrikethrough` | ❌ |
| `` Mod+` `` | Code inline | `` `text` `` | `FormatCode` | ❌ |
| `Mod+L` | Insert link | `[label](url)` | `InsertLink` | ❌ |
| `Ctrl+L` (Mac) / `Alt+L` | Clear block | — | `ClearBlock` | ❌ |
| `Ctrl+U` (Mac) / `Alt+U` | Kill line before cursor | — | `KillLineBackward` | ❌ |
| `Ctrl+K` (Mac) / `Alt+K` | Kill line after cursor | — | `KillLineForward` | ❌ |
| `Ctrl+W` (Mac) / `Alt+D` | Delete word forward | — | `KillWordForward` | ❌ |
| `Alt+W` | Delete word backward | — | `KillWordBackward` | ❌ |
| `Mod+Shift+E` | Copy block embed | `{{embed ((uuid))}}` | `CopyBlockEmbed` | ❌ |
| `Mod+Shift+V` | Paste as single block | — | `PasteSingleBlock` | ❌ |

### 2.4 Block Navigation (selected mode, not editing)

| Shortcut | Action | Intent | Implemented |
|----------|--------|--------|-------------|
| `Up` | Move to previous block | `NavigateUp` | ❌ |
| `Down` | Move to next block | `NavigateDown` | ❌ |
| `Left` | Collapse / go to parent | `NavigateLeft` | ❌ |
| `Right` | Expand / go to first child | `NavigateRight` | ❌ |
| `Alt+Up` | Select block above | `SelectUp` | ❌ |
| `Alt+Down` | Select block below | `SelectDown` | ❌ |
| `Shift+Up` | Select text up (editing) | `SelectTextUp` | ❌ |
| `Shift+Down` | Select text down (editing) | `SelectTextDown` | ❌ |
| `Enter` | Open block for editing | `StartEditing` | ✅ |
| `Shift+Enter` | Open in sidebar | `OpenInSidebar` | ❌ |
| `Mod+Shift+A` | Select all blocks | `SelectAll` | ❌ |
| `Mod+A` | Select parent block | `SelectParent` | ❌ |
| `Backspace` / `Delete` | Delete selected blocks | `DeleteSelection` | ❌ |

### 2.5 Properties (Selection Mode)

| Shortcut | Action | Intent | Implemented |
|----------|--------|--------|-------------|
| `p t` | Set Tags property | `AddProperty(Tags)` | ❌ |
| `p d` | Set Deadline property | `AddProperty(Deadline)` | ❌ |
| `p s` | Set Status property | `AddProperty(Status)` | ❌ |
| `p p` | Set Priority property | `AddProperty(Priority)` | ❌ |
| `Ctrl+Space` | Add comment | `AddComment` | ❌ |

### 2.6 Go-To Navigation (`g` prefix)

| Shortcut | Action | Implemented |
|----------|--------|-------------|
| `g h` | Go home (today's journal) | ❌ |
| `g j` | Go to journals | ❌ |
| `g a` | Go to all pages | ❌ |
| `g g` | Go to graph view | ❌ |
| `g s` | Go to keyboard shortcuts | ❌ |
| `g t` | Go to tomorrow's journal | ❌ |
| `g n` | Go to next journal | ❌ |
| `g p` | Go to previous journal | ❌ |

### 2.7 Toggle UI (`t` prefix)

| Shortcut | Action | Implemented |
|----------|--------|-------------|
| `t b` | Toggle bracket visibility (`[[ ]]`) | ❌ |
| `t d` | Toggle document mode (no bullets) | ❌ |
| `t l` | Toggle left sidebar | ❌ |
| `t r` | Toggle right sidebar | ❌ |
| `t w` | Toggle wide mode | ❌ |
| `t t` | Toggle theme (dark/light) | ❌ |
| `t o` | Toggle open/close all blocks | ❌ |
| `t n` | Toggle number list | ❌ |

### 2.8 Sidebar Shortcuts

| Shortcut | Action | Implemented |
|----------|--------|-------------|
| `Mod+Shift+J` / `Alt+Shift+J` | Open today in sidebar | ❌ |
| `c t` | Close top sidebar panel | ❌ |
| `Mod+C Mod+C` | Clear sidebar | ❌ |

### 2.9 Copy/Paste

| Shortcut | Action | Implemented |
|----------|--------|-------------|
| `Mod+C` | Copy (blocks or text) | ❌ |
| `Mod+Shift+C` | Copy as text | ❌ |
| `Mod+X` | Cut | ❌ |
| `Mod+Shift+E` | Copy block embed | ❌ |
| `Mod+Shift+V` | Paste text in one block | ❌ |

---

## 3. Autocomplete Triggers

| Trigger | Context | Opens | Implemented |
|---------|---------|-------|-------------|
| `[[` | Editing block | Page autocomplete | ⚠️ (infra done, no data) |
| `((` | Editing block | Block autocomplete | ⚠️ (infra done, no data) |
| `#` | Editing block | Tag autocomplete | ⚠️ (infra done, no data) |
| `/` | Editing block | Slash command palette | ⚠️ (trigger detected) |

### Autocomplete Navigation (when dropdown is open)

| Shortcut | Action |
|----------|--------|
| `Up` / `Ctrl+P` | Previous item |
| `Down` / `Ctrl+N` | Next item |
| `Enter` | Select item |
| `Escape` | Cancel |
| `Shift+Enter` | Select and open in sidebar |
| `Mod+Enter` | Select without closing |

---

## 4. Priority and Implementation Order

### MUST (Phase 4)
- Block editing: `Enter`, `Tab`, `Shift+Tab`, `Backspace`, `Escape`
- Global: `Mod+Z`, `Mod+Shift+Z`
- Block navigation: `Up`, `Down`, `Enter`
- Cycle status: `Mod+Enter`

### SHOULD (Phase 4-6)
- Formatting: `Mod+B`, `Mod+I`, `Mod+Shift+H`, `Mod+Shift+S`
- Structural: `Mod+Up`, `Mod+Down`, `Mod+.`, `Mod+,`
- Move: `Mod+Shift+Up`, `Mod+Shift+Down`
- Go-to: `g j`, `g t`, `g p`, `g n`
- Autocomplete nav: `Up`/`Down`/`Enter`/`Escape`

### COULD (post-baseline)
- Kill line/word (`Ctrl+U`, `Ctrl+K`, `Ctrl+W`)
- Toggle UI (`t l`, `t r`, `t d`, `t b`, `t w`, `t t`)
- Properties shortcuts (`p t`, `p d`, `p s`, `p p`)
- Sidebar shortcuts (`c t`, `Mod+C Mod+C`)

---

## 5. Platform Variations

Where Logseq uses different shortcuts per platform, Quilt MUST:

| Action | macOS | Linux/Windows |
|--------|-------|---------------|
| Mod key | `Cmd` | `Ctrl` |
| Zoom in | `Cmd+.` | `Alt+Right` |
| Zoom out | `Cmd+,` | `Alt+Left` |
| Move block up | `Cmd+Shift+Up` | `Alt+Shift+Up` |
| Move block down | `Cmd+Shift+Down` | `Alt+Shift+Down` |
| Clear block | `Ctrl+L` | `Alt+L` |
| Kill line before | `Ctrl+U` | `Alt+U` |
| Kill line after | `Ctrl+K` | `Alt+K` |
| Open today in sidebar | `Cmd+Shift+J` | `Alt+Shift+J` |
