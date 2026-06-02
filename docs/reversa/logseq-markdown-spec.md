# Logseq Markdown Rendering & Editing Specification

> **Source of truth** for replicating Logseq's markdown behavior in Quilt.
> Derived from Logseq source code (`block.cljs`, `mldoc.cljs`, `shortcut/config.cljs`)
> and the `mldoc` npm parser.

---

## 1. Architecture Overview

Logseq uses the `mldoc` npm package for its markdown parser. The AST is a ClojureScript EDN structure
where each node is a vector `["NodeType" ...children]`.

Quilt uses a custom `InlineParser` in Rust (`parser/inline.rs`) that produces `Segment` enum variants.

---

## 2. Inline Formatting (Edit Mode → Display Mode)

### 2.1 Bold — `**text**`

| Aspect | Behavior |
|--------|----------|
| **Syntax** | `**text**` or `__text__` |
| **Keyboard** | `Ctrl+B` / `Cmd+B` |
| **Edit mode** | Raw `**text**` shown |
| **Display mode** | Text rendered in **bold** weight |
| **AST type** | `["Strong" ...]` |
| **Nesting** | Can contain italic, code, links inside |
| **Edge case** | `****` → renders as empty bold (hidden) |
| **CSS** | `font-weight: bold` or `font-weight: 600` |

### 2.2 Italic — `*text*`

| Aspect | Behavior |
|--------|----------|
| **Syntax** | `*text*` or `_text_` |
| **Keyboard** | `Ctrl+I` / `Cmd+I` |
| **Edit mode** | Raw `*text*` shown |
| **Display mode** | Text rendered in *italic* style |
| **AST type** | `["Emph" ...]` |
| **CSS** | `font-style: italic` |

### 2.3 Bold+Italic — `***text***`

| Aspect | Behavior |
|--------|----------|
| **Syntax** | `***text***` or `___text___` |
| **Display mode** | Text rendered in ***bold+italic*** |
| **AST type** | `["Strong" ["Emph" ...]]` or nested |

### 2.4 Strikethrough — `~~text~~`

| Aspect | Behavior |
|--------|----------|
| **Syntax** | `~~text~~` |
| **Keyboard** | `Ctrl+Shift+S` / `Cmd+Shift+S` |
| **Display mode** | Text with ~~strikethrough~~ line |
| **AST type** | `["Strike" ...]` |
| **Quilt status** | ❌ Not implemented |

### 2.5 Highlight — `^^text^^`

| Aspect | Behavior |
|--------|----------|
| **Syntax** | `^^text^^` |
| **Keyboard** | `Ctrl+Shift+H` / `Cmd+Shift+H` |
| **Display mode** | Text with background highlight (yellow/green) |
| **AST type** | `["Highlight" ...]` |
| **Quilt status** | ❌ Not implemented |

### 2.6 Inline Code — `` `code` ``

| Aspect | Behavior |
|--------|----------|
| **Syntax** | `` `code` `` |
| **Keyboard** | `` Ctrl+` `` / `` Cmd+` `` |
| **Display mode** | Monospace font with subtle background |
| **AST type** | `["Code" ...]` |
| **Quilt status** | ✅ Implemented (`Segment::Code`) |

### 2.7 Links — `[text](url)`

| Aspect | Behavior |
|--------|----------|
| **Syntax** | `[display text](https://url.com)` |
| **Keyboard** | `Ctrl+L` / `Cmd+L` (insert link template) |
| **Display mode** | Blue underlined `display text`, clickable |
| **Click** | Opens URL in new tab |
| **AST type** | `["Link" {:url ["URL" "https://..."] :label [...]}]` |
| **Quilt status** | ✅ Implemented (`Segment::Link`) |

### 2.8 Wikilinks — `[[Page Name]]`

| Aspect | Behavior |
|--------|----------|
| **Syntax** | `[[Page Name]]` or `[[Page Name|alias]]` |
| **Trigger** | Type `[[` → autocomplete dropdown |
| **Display mode** | Green underlined page name, clickable |
| **Click** | Navigates to referenced page |
| **Non-existent page** | Red link, creates page on click |
| **AST type** | `["Link" {:url ["Page_ref" "page-name"]}]` |
| **Alias syntax** | `[[Real Page|Display Text]]` — shows alias in display |
| **Quilt status** | ✅ Implemented (`Segment::PageRef`) |

### 2.9 Block References — `((block-uuid))`

| Aspect | Behavior |
|--------|----------|
| **Syntax** | `((uuid-string))` |
| **Trigger** | Type `((` → autocomplete dropdown |
| **Display mode** | Inline embed of referenced block content with background |
| **AST type** | `["Block_reference" "uuid"]` or `["Macro" {:name "embed"}]` |
| **Quilt status** | ✅ Implemented (`Segment::BlockRef`) |

### 2.10 Tags — `#tagname`

| Aspect | Behavior |
|--------|----------|
| **Syntax** | `#tagname` or `#multi-word-tag` |
| **Trigger** | Type `#` → autocomplete dropdown |
| **Display mode** | Colored pill/badge with `#` prefix |
| **AST type** | `["Tag" "tagname"]` |
| **Quilt status** | ✅ Implemented (`Segment::Tag`) |

### 2.11 Hashtags (nested) — `#parent/nested`

| Aspect | Behavior |
|--------|----------|
| **Syntax** | `#namespace/tag` |
| **Display mode** | Hierarchical tag with breadcrumb style |
| **AST type** | `["Tag" "namespace/tag"]` |
| **Quilt status** | ❌ Not implemented |

---

## 3. Block-Level Formatting

### 3.1 Headers — `# H1`, `## H2`, ... `###### H6`

| Aspect | Behavior |
|--------|----------|
| **Syntax** | `# ` to `###### ` at start of block |
| **Display mode** | Large bold text. H1 = 1.8em, H2 = 1.5em, H3 = 1.3em, etc. |
| **AST type** | `["Heading" {:size 1-6} ...]` |
| **Properties** | Heading stored in `block/properties :heading size` |
| **Edit mode** | Shows raw `# ` prefix |
| **Collapse** | Has children? Collapse behavior same as non-heading |
| **Quilt status** | ⚠️ Partial — parsed but not rendered with heading styles |

### 3.2 Block Quotes — `> text`

| Aspect | Behavior |
|--------|----------|
| **Syntax** | `> text` or `> > nested` |
| **Display mode** | Left border bar (2-3px), slightly muted text |
| **AST type** | `["Blockquote" ...]` |
| **Quilt status** | ❌ Not implemented |

### 3.3 Code Blocks — `` ```lang ``

| Aspect | Behavior |
|--------|----------|
| **Syntax** | `` ```language\ncode\n``` `` |
| **Display mode** | Monospace with syntax highlighting, copy button |
| **AST type** | `["Code_block" {:lang "js"} "..."]` |
| **Wrap** | Horizontal scroll if long lines |
| **Quilt status** | ❌ Not implemented |

### 3.4 Unordered Lists — `- item`

| Aspect | Behavior |
|--------|----------|
| **Syntax** | `- item` or `* item` or `+ item` |
| **Display mode** | Bullet points (same bullet as blocks) |
| **AST type** | `["Unordered_list" [...items]]` |
| **Quilt status** | N/A — Quilt blocks ARE the list items |

### 3.5 Ordered Lists — `1. item`

| Aspect | Behavior |
|--------|----------|
| **Syntax** | `1. item` (auto-numbers) |
| **Keyboard** | `tn` (toggle number list) |
| **Display mode** | Numbered items |
| **AST type** | `["Ordered_list" [...items]]` |
| **Quilt status** | ❌ Not implemented |

### 3.6 Horizontal Rules — `---`

| Aspect | Behavior |
|--------|----------|
| **Syntax** | `---`, `***`, or `___` on own line |
| **Display mode** | Horizontal line across content width |
| **Quilt status** | ❌ Not implemented |

### 3.7 Images — `![alt](url)`

| Aspect | Behavior |
|--------|----------|
| **Syntax** | `![alt text](url)` or `![alt](../assets/image.png)` |
| **Display mode** | Rendered image, max-width 100% |
| **AST type** | `["Image" {:url ["URL" "..."] :label ["Plain" "alt"]}]` |
| **Quilt status** | ❌ Not implemented |

---

## 4. Properties — `key:: value`

### 4.1 Basic properties

| Aspect | Behavior |
|--------|----------|
| **Syntax** | `key:: value` at start of block content |
| **Display mode** | Key in muted color, value in normal color. Key:Value pair shown inline. |
| **AST type** | `["Paragraph" ["Property" "key::" " value"]]` |
| **Multiple** | Can have multiple `key1:: v1 key2:: v2` in one block |
| **Edit mode** | Raw `key:: value` shown |
| **Hidden flag** | `collapsed:: true` → block starts collapsed |
| **Public flag** | `public:: true` |
| **Icon** | `icon:: 📌` |
| **Template** | `template:: template-name` |
| **Alias** | `alias:: alias1, alias2` |
| **Tags** | `tags:: tag1, tag2` |
| **ID** | `id:: uuid` |
| **Quilt status** | ✅ Implemented (`Segment::Property`) |

### 4.2 Property Render Types

| Type | Syntax | Display |
|------|--------|---------|
| **Status** | `status:: todo/doing/done` | Colored badge with icon |
| **Priority** | `priority:: A/B/C` | Colored priority badge |
| **Deadline** | `deadline:: YYYY-MM-DD` | Date with overdue highlight |
| **Scheduled** | `scheduled:: YYYY-MM-DD` | Date |
| **Quilt status** | ⚠️ Partially (status/priority rendering exists) |

---

## 5. Markers — TODO / DOING / DONE / LATER / NOW

### 5.1 Marker Rendering

| Marker | Display | Icon |
|--------|---------|------|
| **TODO** | Orange text | ○ open circle |
| **DOING** | Blue text | ◐ half-filled |
| **DONE** | Green text with strikethrough | ✓ checkmark |
| **LATER** | Gray text | ⏳ hourglass |
| **NOW** | Red text | ● filled circle |
| **CANCELLED** | Gray text with strikethrough | ✕ |
| **WAITING** | Purple text | ⏸ pause |

### 5.2 Marker keyboard

| Action | Key |
|--------|-----|
| **Cycle TODO/DOING/DONE** | `Ctrl+Enter` / `Cmd+Enter` |
| **Set via slash** | `/TODO` `/DOING` `/DONE` `/LATER` `/NOW` |
| **Set via right-click** | Context menu → Marker |

### 5.3 Quilt status

| Feature | Status |
|---------|--------|
| Marker field in BlockDto | ✅ |
| Cycle via `Ctrl+Enter` | ✅ (UI only, backend pending) |
| Slash commands for markers | ✅ |
| Marker display in static view | ⚠️ Partial (circle icon shown, no color) |
| Strikethrough for DONE | ❌ |
| Context menu | ❌ |

---

## 6. Macros

### 6.1 Template Macros — `{{macro}}`

| Macro | Behavior |
|-------|----------|
| `{{embed ((uuid))}}` | Embed referenced block content inline |
| `{{video url}}` | Embed video |
| `{{twitter url}}` | Embed tweet |
| `{{youtube url}}` | Embed YouTube video |
| `{{renderer name}}` | Custom renderer |
| `{{query (task TODO)}}` | Dynamic query |

**AST type**: `["Macro" {:name "embed" :arguments [...]}]`

**Quilt status**: ❌ Not implemented

---

## 7. Autocomplete Triggers

| Trigger | Opens | On select |
|---------|-------|-----------|
| `[[` | Page reference autocomplete | Insert `[[Page Name]]` |
| `((` | Block reference autocomplete | Insert `((uuid))` |
| `#` | Tag autocomplete | Insert `#tagname` |
| `/` | Slash command menu | Execute command |
| `[` (after typing) | Link autocomplete | Insert `[text](url)` |

### 7.1 Autocomplete details

| Trigger | Filter | Sort | Max Items |
|---------|--------|------|-----------|
| `[[` | Page names matching prefix | Most recent / alphabetical | 10 |
| `((` | Block content matching prefix | Most recent | 10 |
| `#` | Tag names matching prefix | Frequency | 10 |
| `/` | Command names | Category grouped | All |

### 7.2 Quilt autocomplete status

| Trigger | Status |
|---------|--------|
| `[[` → page ref | ⚠️ Pipeline exists, needs testing |
| `((` → block ref | ⚠️ Pipeline exists, needs testing |
| `#` → tag | ⚠️ Pipeline exists, needs testing |
| `/` → slash | ✅ Working |

---

## 8. Edit Mode vs Display Mode Contrast

### 8.1 What changes visually

| Element | Edit Mode | Display Mode |
|---------|-----------|--------------|
| `**bold**` | Raw `**bold**` markers visible | Bold text, markers hidden |
| `*italic*` | Raw `*italic*` markers visible | Italic text, markers hidden |
| `~~strike~~` | Raw `~~strike~~` markers visible | Strikethrough, markers hidden |
| `^^highlight^^` | Raw `^^highlight^^` markers visible | Highlight, markers hidden |
| `` `code` `` | Raw `` `code` `` markers visible | Monospace bg, markers hidden |
| `[text](url)` | Raw `[text](url)` markers visible | Blue link, markers hidden |
| `[[Page]]` | Raw `[[Page]]` markers visible | Green wikilink, markers hidden |
| `((uuid))` | Raw `((uuid))` markers visible | Inline embed, markers hidden |
| `#tag` | Raw `#tag` markers visible | Tag pill, `#` visible |
| `key:: value` | Raw `key:: value` markers visible | Styled property |
| `# Heading` | Raw `# ` prefix visible | Large text, `# ` hidden |

### 8.2 CM6 Extensions (how Logseq does it)

Logseq uses CM6 with custom decorations to hide markdown markers in edit mode:
- **Mark decoration**: Hides the `**` markers while keeping them in the document
- **Replace decoration**: Replaces `**text**` with styled bold text
- **Widget decoration**: For complex elements like block embeds

This means in Logseq's edit mode:
1. The actual document contains `**bold text**`
2. CM6 decorations hide the `**` and style "bold text" as bold
3. Cursor movement treats markers as invisible but present

---

## 9. Quilt Implementation Status & Gaps

### 9.1 Implemented ✅

| Feature | Parser | Display | Keyboard |
|---------|--------|---------|----------|
| **Bold** `**text**` | ✅ | ✅ `.md-bold` | ❌ |
| **Italic** `*text*` | ✅ | ✅ `.md-italic` | ❌ |
| **Inline Code** `` `code` `` | ✅ | ✅ `.md-code` | ❌ |
| **Links** `[text](url)` | ✅ | ✅ `.md-link` | ❌ |
| **Wikilinks** `[[Page]]` | ✅ | ✅ | Partial (autocomplete) |
| **Block refs** `((uuid))` | ✅ | ✅ | Partial |
| **Tags** `#tag` | ✅ | ✅ | Partial |
| **Properties** `key:: value` | ✅ | ✅ | ❌ |
| **Markers** TODO/DOING/DONE | ✅ | Partial | ✅ `Ctrl+Enter` (UI) |

### 9.2 Not Implemented ❌

| Feature | Priority | Effort |
|---------|----------|--------|
| **Strikethrough** `~~text~~` | P1 | Low — add Segment + CSS |
| **Highlight** `^^text^^` | P1 | Low — add Segment + CSS |
| **Bold+Italic** `***text***` | P2 | Low — nested segments |
| **Headers** `# ## ###` | P1 | Medium — block-level CSS |
| **Block Quotes** `>` | P2 | Medium |
| **Code Blocks** `` ``` `` | P2 | High |
| **Images** `![]()` | P2 | Medium |
| **Ordered Lists** `1.` | P3 | High |
| **Horizontal Rules** `---` | P3 | Low |
| **Macros** `{{macro}}` | P3 | Very High |
| **CM6 decorations** (hide markers in edit) | P1 | High — CM6 extension |
| **Marker colors** (TODO=orange, DONE=green strikethrough) | P1 | Low — CSS |
| **Formatting shortcuts** (Ctrl+B, Ctrl+I, etc.) | P1 | Medium — CM6 keybindings |

### 9.3 Priority Backlog

**P0 — Core markdown feel:**
1. Strikethrough `~~text~~` — add `Segment::Strikethrough`
2. Highlight `^^text^^` — add `Segment::Highlight`
3. Marker colors + DONE strikethrough — CSS only
4. Headers `# ## ###` — block-level CSS sizing

**P1 — Formatting shortcuts:**
5. `Ctrl+B` / `Cmd+B` → toggle bold
6. `Ctrl+I` / `Cmd+I` → toggle italic
7. `Ctrl+Shift+S` → toggle strikethrough
8. `Ctrl+Shift+H` → toggle highlight
9. `` Ctrl+` `` → toggle inline code

**P2 — CM6 integration:**
10. CM6 decorations to hide markdown markers in edit mode
11. Block quotes `>`
12. Images `![]()`

**P3 — Advanced:**
13. Code blocks with syntax highlighting
14. Ordered lists
15. Macros `{{}}`

---

## 10. CSS Reference

### 10.1 Display mode CSS classes (Quilt)

```css
.md-bold     { font-weight: 600; }
.md-italic   { font-style: italic; color: var(--color-teal); }
.md-code     { font-family: monospace; background: var(--color-bg-code); padding: 1px 4px; border-radius: 3px; }
.md-link     { color: var(--color-accent); text-decoration: underline; cursor: pointer; }
.md-strike   { text-decoration: line-through; opacity: 0.7; }
.md-highlight { background-color: var(--color-highlight); padding: 0 2px; }
.page-ref    { color: var(--color-green); cursor: pointer; }
.block-ref   { background: var(--color-bg-ref); border-left: 3px solid var(--color-accent); padding: 2px 8px; }
.tag-pill    { background: var(--color-bg-tag); color: var(--color-tag); padding: 0 4px; border-radius: 4px; font-size: 0.85em; }
.property-key { color: var(--color-muted); font-size: 0.9em; }
.property-val { color: var(--color-text); }
```

### 10.2 Marker color reference

```css
.marker-todo       { color: #D97706; } /* orange-600 */
.marker-doing      { color: #2563EB; } /* blue-600 */
.marker-done       { color: #16A34A; text-decoration: line-through; } /* green-600 */
.marker-later      { color: #6B7280; } /* gray-500 */
.marker-now        { color: #DC2626; } /* red-600 */
.marker-cancelled  { color: #9CA3AF; text-decoration: line-through; }
.marker-waiting    { color: #7C3AED; } /* purple-600 */
```

### 10.3 Heading sizes

```css
.h1 { font-size: 1.8em; font-weight: 700; margin: 0.5em 0; }
.h2 { font-size: 1.5em; font-weight: 600; margin: 0.4em 0; }
.h3 { font-size: 1.3em; font-weight: 600; margin: 0.3em 0; }
.h4 { font-size: 1.1em; font-weight: 600; }
.h5 { font-size: 1.0em; font-weight: 600; }
.h6 { font-size: 0.9em; font-weight: 600; color: var(--color-muted); }
```

---

## 11. Backend Considerations

### 11.1 Block Content Storage

Logseq stores the RAW markdown in `block/title` (or `block/content`). Quilt stores raw markdown in `block.content`. This is correct.

### 11.2 Properties Storage

Logseq stores properties in a separate `block/properties` map, NOT in the content. Quilt currently keeps properties inline in `block.content`. This needs to be aligned:

| Approach | Pros | Cons |
|----------|------|------|
| **Inline** (current Quilt) | Simple, no migration | Duplicate parsing, harder to query |
| **Separate** (Logseq) | Queryable, normalized | Migration needed, sync between content and properties |

**Recommendation**: Keep inline for MVP, migrate to separate properties table in v2.

### 11.3 Full-text Search (FTS)

Logseq indexes the RENDERED (plain text) version of content for FTS. Quilt's FTS5 index indexes raw markdown. Consider indexing rendered text for better search results.

---

## 12. E2E Test Matrix for Markdown

```yaml
bold:
  - type **text** and verify bold rendering in display mode
  - verify Ctrl+B toggles bold markers
  - verify bold inside italic: *_text_*
  - verify bold at boundaries: **start** middle **end**

italic:
  - type *text* and verify italic rendering
  - verify Ctrl+I toggles
  - verify `*` alone doesn't trigger (not followed by non-space)

strikethrough:
  - type ~~text~~ and verify strikethrough
  - verify Ctrl+Shift+S

code:
  - type `code` and verify monospace rendering
  - verify code with backticks inside

links:
  - type [text](url) and verify link rendering
  - verify click opens URL
  - verify Ctrl+L inserts link template

wikilinks:
  - type [[ and verify autocomplete
  - verify [[Page Name]] renders as green link
  - verify click navigates to page

tags:
  - type #tag and verify pill rendering
  - verify #multi-word-tag
  - verify #nested/tag hierarchy

headers:
  - type # H1 and verify large text
  - type ## H2, ### H3 etc.

properties:
  - type key:: value and verify styled rendering
  - verify multiple properties in one block
  - verify status:: auto-converts to badge
  - verify priority:: A/B/C

markers:
  - verify TODO block shows orange
  - verify DONE block shows green + strikethrough
  - verify Ctrl+Enter cycles through TODO→DOING→DONE→None

edit_vs_display:
  - verify ** markers hidden/semi-transparent in edit mode
  - verify switching between edit and display preserves formatting
  - verify save and reload preserves all formatting
```
