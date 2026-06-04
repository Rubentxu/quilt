# Exploration: External Property System Research — Notion, AnyType, Logseq

> **Purpose**: Deep-dive into how Notion, AnyType, and Logseq implement their property systems — concrete UI patterns, data models, and AI agent interaction patterns for Quilt to learn from.

---

## Current State (Quilt)

Quilt currently defines properties in `quilt-domain` as typed values with a `PropertyType` enum. The system supports:
- Basic scalar types (text, number, checkbox, date)
- Select/multi-select with predefined options
- Links (page references)
- File attachments

Properties are stored in SQLite and exposed via the REST API. The frontend (`quilt-ui`) renders properties in a basic key-value layout. There is no layout builder, no property grouping, no pinned properties, no side panel, and no template-based property pre-fill.

---

## Affected Areas (in Quilt)

- `crates/quilt-domain/src/property.rs` — Property value types and definitions
- `crates/quilt-infrastructure/src/sqlite/` — Property storage schema
- `quilt-ui/src/features/properties/` — Frontend property rendering
- `quilt-ui/src/features/outliner-tiptap/` — Block-level properties
- `crates/quilt-mcp/` — MCP property tools for AI agents
- `crates/quilt-server/` — REST API property endpoints

---

## 1. NOTION — Property System Deep Dive

### 1.1 Property Layout Builder (Layouts)

**Exact UI Flow:**
1. Open any database page → Click `···` top-right → `Customize layout`
2. This opens a **builder modal** with three target zones highlighted with blue borders:
   - **Heading** (top, under page title)
   - **Property Group** (main content area, below heading)
   - **Details Panel** (right sidebar, hidden by default)

**Three Layout Zones:**

| Zone | Max Items | Layout | Behavior |
|------|-----------|--------|----------|
| **Pinned** (Heading) | 4 properties | Horizontal row under page title | Always visible. Once pinned, property disappears from Property Group |
| **Property Group** | Unlimited | Vertical stack, can have collapsible **sections** | Default home for all properties. Sections act like toggles — expand/collapse groups of properties |
| **Panel** (right sidebar) | Unlimited | Right sidebar, toggle open/close | Hidden by default. Properties here can be styled "Large" or "Small" |

**Key UX details:**
- Properties are **exclusive** to one zone — moving to Panel/Pinned removes from Property Group
- Sections within Property Groups are created via "`+ Add section`" button in the right sidebar of the builder
- Relation properties shown as "Page section" get a special layout — listed as linked page cards
- "Apply to all pages" blue button saves layout for entire database
- Property icons can be toggled off globally

**Screenshot reference**: See before/after comparison at [Simone Smerilli's article](https://www.simonesmerilli.com/life/notion-layouts) — the transformation from flat vertical stack to organized zones is dramatic.

### 1.2 Complete Property Types (22 types)

From the Notion API `property-object` schema (developers.notion.com):

| # | Type | Data Kind | Writable | UI Rendering |
|---|------|-----------|----------|--------------|
| 1 | `title` | String (rich text) | Yes | Primary name field, required |
| 2 | `rich_text` | String | Yes | Plain text, supports formatting |
| 3 | `number` | Number | Yes | Formatted as number/currency/percent/progress bar |
| 4 | `select` | String (single) | Yes | Colored tag pill (10 colors) |
| 5 | `multi_select` | String[] (multiple) | Yes | Multiple colored tag pills |
| 6 | `status` | String (single) | Yes | Tag pill with To-do/In Progress/Complete groups |
| 7 | `date` | Date (range optional) | Yes | Date picker popup, calendar view support |
| 8 | `people` | Person[] | Yes | Avatar + name chips |
| 9 | `files` | File[] | Yes | Thumbnail previews for images, file icons for others |
| 10 | `checkbox` | Boolean | Yes | Check/uncheck toggle |
| 11 | `url` | String | Yes | Clickable link |
| 12 | `email` | String | Yes | mailto: link |
| 13 | `phone` | String | Yes | tel: link on mobile |
| 14 | `formula` | Computed | Read-only | Dynamic computed value, any output type |
| 15 | `relation` | Page ref(s) | Yes | Linked page chips, two-way sync optional |
| 16 | `rollup` | Computed | Read-only | Aggregation across relation (sum/avg/count/etc.) |
| 17 | `created_time` | Date | Read-only | Auto-set on creation |
| 18 | `created_by` | Person | Read-only | Auto-set on creation |
| 19 | `last_edited_time` | Date | Read-only | Auto-updated on edits |
| 20 | `last_edited_by` | Person | Read-only | Auto-updated on edits |
| 21 | `unique_id` | String | Read-only | Auto-incrementing ID with optional prefix |
| 22 | `place` | Location | Yes | Map pin + address via location services |

**AI-Enabled Property Types (Autofill):**
- `AI Summary` — auto-summarizes page content into the property
- `AI Key Info` — extracts defined key information
- `AI Custom Autofill` — user-defined prompt, uses page context
- `AI Translation` — translates one property to another language

**"+ Add Property" Flow:**
1. Click `+` next to rightmost property in table view (or `+ Add property` in page header)
2. Modal: Type name → dropdown of types with icons → select type
3. Alternatively: right-click property header → `Insert left` / `Insert right`
4. In 2.52 (July 2025): refreshed UI with "the new `+ property` makes naming and picking a property much smoother"

**Validation per type (from API and UI):**
- Select/Multi-select: option names must be unique (case-insensitive), commas not valid in option names
- Status: grouped into To-do/In Progress/Complete categories, each category has color options
- Date: accepts ISO 8601, supports date ranges (start + optional end)
- Number: format specified separately (number, dollar, euro, pound, yen, ruble, rupee, won, yuan, real, lira, rupiah, franc, hong_kong_dollar, new_zealand_dollar, krona, norwegian_krone, mexican_peso, rand, new_taiwan_dollar, danish_krone, zloty, baht, forint, koruna, shekel, chilean_peso, philippine_peso, dirham, colombian_peso, riyal, ringgit, turkish_lira, argentine_peso, peruvian_sol) + percent
- Relation: must reference a database, Notion auto-creates reciprocal relation
- Unique ID: cannot be manually changed, auto-increments, prefixes allowed

### 1.3 Property Rendering

**Inline rendering in page header:**
- **Select**: Colored rounded pill with text. 10 colors, randomly assigned. Hover shows `···` to edit color/name.
- **Multi-select**: Multiple colored pills side by side.
- **Status**: Like Select but with predefined group coloring (To-do=gray, In Progress=blue, Complete=green).
- **Date**: Text showing date with calendar icon. Click opens date picker popup with month view.
- **Person**: Circular avatar + name. Hover shows full profile card.
- **Relation**: Rounded pill with page icon. Click navigates to linked page. Shows page title.
- **Formula**: Plain text or number with formatting. Computed display only.
- **Checkbox**: Actual checkbox toggle, not text.
- **Files & Media**: Thumbnail preview (images), file type icon + name (others). Drag-to-reorder with `⋮⋮`.
- **URL**: Underlined clickable link.

**Property descriptions (tooltips):**
- Click property name → Edit property → little `i` icon next to name → "Add property description"
- Descriptions appear as tooltip on hover in table views and page header

### 1.4 Page Header Properties

The page header for a database item shows properties according to the **Customize layout** configuration:
- **Pinned** (max 4): horizontal row immediately under page title. Always visible.
- **Property Group**: collapsible sections, vertically stacked. Default if not customized.
- **Panel** (right): hidden by default, toggle with panel button.

Backlinks and comments can also be toggled on/off in the Heading configuration.

### 1.5 Property Groups / Sections

- Property Groups can contain **sections** — collapsible dividers (like toggles)
- Sections have labels (e.g., "Task Details", "Project Info")
- Properties are dragged into sections via the Layout Builder sidebar
- Users can create **multiple property groups** and sections
- A property group containing only relation properties can be shown as **"minimal"** or **"page section"** (the latter shows linked pages as cards)

### 1.6 Database Views — How Properties Appear

| View | Property Display |
|------|-----------------|
| **Table** | Columns. Conditional color on cells (by property value). Properties can be shown/hidden per view via `Property visibility`. Drag to reorder columns. |
| **Board** (Kanban) | Cards grouped by Select/Status/Person property. Card shows: cover image/preview + title + visible properties below. Card size: Small/Medium/Large. |
| **Gallery** | Cards with image preview. `Card preview` can be: page cover, Files & Media property, or page content. Visible properties listed below preview. |
| **Calendar** | Items shown as bars on date cells. Uses Date property for placement. "Show calendar by" to pick which date property. |
| **Timeline** | Gantt-like bars. Requires Date property with range. Items shown horizontally across time axis. |
| **List** | Compact rows. Properties shown inline. Good for minimal views. |
| **Feed** (new 2025) | Blog-feed style. Scroll through items with comments and reactions. |
| **Chart** | Visual charts (bar, line, donut) aggregating property values. |

**Conditional color**: Applied per view. Rules like "If Status = Done → green background". Works on Table, Calendar, Timeline, List, Board, Feed views.

---

## 2. ANYTYPE — Property System Deep Dive

### 2.1 Object Types Define Property Sets

**Fundamental difference from Notion**: AnyType is **object-type-centric**, not database-centric.

- Every object has a **Type** (similar to a class/template)
- Types define which **Properties** are available
- Properties are **reusable** across types (global property library)
- Types are defined in `Channel Settings > Content Model > Object Types`

**Type Definition Flow:**
1. Create Type (e.g., "Book", "Task", "Person")
2. Choose layout (Basic, Profile, Note, etc.)
3. Add properties from global Property library or create new ones
4. Add Templates to the Type

**Query/Set Views**: Instead of "database views", AnyType uses **Queries** (live filters) and **Sets** (collections). A Query by Type shows all objects of that Type. Properties are toggled on/off per Query view, not per Type.

### 2.2 Two-Way Linked Properties

**This is AnyType's conceptual advantage over Notion.**

**Concrete example — Book ↔ Author:**
1. Create a `Book` Type with property `Author` (type: Object → links to Person)
2. Create a `Person` Type with property `Books Written` (type: Object → links to Book)
3. In property settings → "Two-way linking" section → link `Author` ↔ `Books Written`
4. **Result**: When you set "J.R.R. Tolkien" as Author on "The Lord of the Rings":
   - The Book page shows Author: J.R.R. Tolkien
   - The Person page for Tolkien **automatically** shows Books Written: The Lord of the Rings
   - The link is removed from backlinks and placed into the property — keeps backlinks cleaner

**Setup UI:**
- Open property settings → "Two-way linking" section
- Choose (or create) the corresponding property in the linked object type
- The connection is handled automatically

**Limitations:**
- Only works between **custom object types** (not built-in types like File, Weblink, Image)
- Single vs. Multiple selection configurable per property

### 2.3 Property Rendering in Object View

**Properties Panel**: Bullet-list icon in top-right corner. Opens panel showing all properties.

**Property Visibility Zones (Type-level configuration):**

| Zone | Description | Visibility |
|------|-------------|------------|
| **Header** | Appears in header part of every object | Always visible (similar to Notion's "Pinned") |
| **Panel** | Shows when Properties icon is clicked | Toggleable via Properties icon |
| **Hidden** | Under "Hidden" toggle in Panel | Hidden unless user expands Hidden section |
| **Local** | Not associated with the Object's Type | Ad-hoc properties on individual objects only |

**Available Property Types (9 types):**
1. Text — freeform
2. Number — all numbers (formatting "coming soon")
3. Date — with optional time
4. Select — single choice from predefined options
5. Multi-select — multiple choices, no limit
6. Email/Phone/URL — special format types
7. Checkbox — boolean
8. File & Media — audio, video, images
9. Object — reference to another object (this is their "Relation" equivalent — actually a link)

### 2.4 Template-Property Binding

**Template creation:**
- Navigate to Type → Templates → `+` → name template → add properties with pre-filled values → auto-saved
- Or: from existing object → `···` → "Use as Template"

**Pre-fill behavior:**
- Templates pre-fill property values (e.g., a "Fiction Book" template sets Genre to "Fiction")
- **Template Name Pre-fill** toggle (new): controls whether template name becomes the object name
- When a template is applied to a new object, pre-filled properties are already set
- Templates live at the Type level; each Type can have multiple templates

### 2.5 Customization — Property Visibility Per Object

- Property visibility is configured at the **Type** level (not per-object)
- Four zones: Header, Panel, Hidden, Local
- Local properties are ad-hoc — added on individual objects without modifying the Type definition
- Users can toggle visibility of properties in Queries (filters on/off per column)
- No per-object customization of which Type-level properties are visible (unlike Notion where you can hide properties per-layout)

---

## 3. LOGSEQ — Property System Deep Dive

### 3.1 Page Properties (Frontmatter)

**Syntax:**
```
title:: Meeting Notes
tags:: #meeting, #team
date:: 2024-12-12
```
- Placed at the **very beginning of a page** (first block, no bullet marker)
- Uses `key:: value` syntax (double colon with no space before colons)
- Backward compatible with YAML frontmatter (`---` blocks) — merged into page properties
- Multiple values separated by commas: `tags:: tag1, tag2`
- Pages referenced as `[[Page Name]]` become page links

**Rendering:**
- Page properties **do not appear in the page body** by default in view mode
- They're visible in edit mode
- In DB version: properties displayed differently (more structured)
- The `Properties.md` docs page itself uses page properties as example

**Key distinction from YAML:** Logseq page properties use `key:: value`, not `key: value`. YAML blocks (`---`) are supported for compatibility.

### 3.2 Block Properties

**Syntax:**
```
- TODO Insert example task
  headspace-required:: low
  priority:: high
  deadline:: 2024-12-25
```

- Attached to any **bullet block** (list item)
- `key:: value` on its own line within the block
- Must be **contiguous** (all properties in a row, no content between them)
- After the last property, content can continue on next lines
- Logseq internal properties use dot-prefix for hidden: `.collapsed:: true`, `.id:: 1234-5677`

**Rendering:**
- Block properties are visible in edit mode as raw `key:: value` text
- In view mode: some are hidden (internal `.` prefixed ones)
- No special visual rendering — plain text
- **Proposed** (not implemented): `[[key:: value]]` inline syntax for embedding properties within sentence text

**Querying block properties:**
```clojure
# Simple query
{{query (property headspace-required low)}}

# Advanced Datalog query
#+BEGIN_QUERY
{:query [:find (pull ?b [*])
         :where
         [?b :block/properties ?props]
         [(get ?props :type) ?type]
         [(= ?type "meeting")]]
}
#+END_QUERY
```

### 3.3 Property Namespaces

**No native namespace/dot-notation support.**

- Logseq does NOT support `namespace.key:: value` syntax
- Property keys are flat strings
- **Workaround**: Use hierarchical page names with `/` as delimiter (e.g., `projects/catalog/registry`)
- **Proposal exists**: dotted properties (`.collapsed::`) already used internally for hidden metadata
- Community discussion about `.property` syntax for hidden user properties
- Underscores in property keys get converted to hyphens in the database (`item_page` → `item-page`)

### 3.4 Property Queries

**Simple queries (DSL):**
```
{{query (page-property status draft)}}
{{query (property status idea)}}
{{query (and (task TODO) (property headspace-required low))}}
```

**Advanced queries (Datalog):**

Pages with specific property:
```clojure
#+BEGIN_QUERY
{:query [:find (pull ?p [*])
         :in $ ?status
         :where
         [?p :block/properties ?props]
         [(get ?props :status) ?s]
         [(= ?s ?status)]]
 :inputs ["draft"]}
#+END_QUERY
```

Blocks with property containing specific page reference:
```clojure
#+BEGIN_QUERY
{:query [:find (pull ?b [*])
         :in $ ?participant
         :where
         [?b :block/properties ?prop]
         [(get ?prop :type) ?type]
         [(= ?type #{"meeting"})]
         [(get ?prop :participants) ?participants]
         [(contains? ?participants ?participant)]]
 :inputs ["Frozen"]}
#+END_QUERY
```

**Gotchas:**
- Property values stored as **sets** (`#{"My Page"}`) — use `(first …)` for scalar access
- `:block/name` is normalized to lowercase
- The keyword in queries must match the stored key (hyphens after underscore conversion)
- Pages have `:block/name`; blocks have `:block/properties`
- `:block/pre-block` identifies page-property blocks vs regular blocks

### 3.5 Property Rendering

**Minimal visual rendering.** Properties are primarily plain text:
- No colored tags, no date pickers, no avatar chips
- `[[Page]]` references are clickable links
- `#tag` references become page links if "process as pages" is configured
- DB version adds some visual improvements (icons, better layout)

**Configurable behavior:**
- In `config.edn`: can specify properties to always hide
- Community plugin "Awesome UI" shows hidden properties
- Property keys can be made clickable to navigate to a page showing all blocks with that property

### 3.6 macOS/iOS Integration

**Desktop (macOS):**
- Full property editing support — both page and block properties
- Properties displayed in both edit and view modes
- DB version has improved property rendering

**Mobile (iOS):**
- **Significant issues** — properties not displayed on iPhone in portrait mode
- Workaround: rotate to landscape mode to see properties (but keyboard space is then too small)
- iOS app is "capture-focused" — less capable than desktop
- Recent DB changelog shows: "Display property pairs vertically on mobile for better readability", "Fix properties not shown on page view", "Fix number property cannot be edited if value is empty"
- Sync via iCloud (with known issues) or Logseq Sync (~$5/mo)

---

## 4. CROSS-CUTTING: AI Agent Property Interaction

### 4.1 Notion — AI Agent Interaction

**Notion REST API (agents can use directly):**
- Database property schema fully exposed via `GET /databases/{id}` → `.properties` object
- Each property object has: `id`, `name`, `type`, and type-specific config (e.g., `select.options`, `number.format`)
- Agent discovers available property types by iterating `Object.entries(database.properties)`
- Property values set via `POST /pages` (create) or `PATCH /pages/{id}` (update) with type-specific JSON format
- **Validation**: API returns `400 validation_error` with specific message if property value doesn't match type

**Notion MCP Server (hosted at `mcp.notion.com`):**
- Tools: `notion-search`, `notion-fetch`, `notion-create-pages`, `notion-update-page`, `notion-query-database`
- Agent discovers database schema through `notion-fetch` on database
- Property values set as native JSON objects (not strings)
- Covers ~80% of practical agent workflows

**Notion Agent (built-in AI):**
- Can read/write properties as part of multi-step tasks
- "Custom Agent Autofill" — user-defined prompts that fill properties using workspace context + web search
- Can query databases with specific property filters: "Which accounts in @Sales CRM have call notes mentioning European data residency?"

**Notion Custom Agents (via API):**
- External tools can be registered as agent tools that Notion agents call
- Workers define input schemas validated by Notion

**Property type discovery pattern (for Quilt to adopt):**
```javascript
// Agent discovers database schema
const db = await notion.databases.retrieve({ database_id });
for (const [name, prop] of Object.entries(db.properties)) {
  console.log(`${name}: ${prop.type}`);  
  // e.g., "Status: select", "Priority: select", "Deadline: date"
}
// Agent then validates its output against discovered schema
```

### 4.2 Logseq — AI Agent Interaction

**MCP Server (community):**
- `mcp-logseq` — tools: `list_pages`, `get_page_content`, `create_page`, `update_page`, `search`, `query`, `find_pages_by_property`
- Works with Logseq API token + DB mode
- Property support via `find_pages_by_property` — searches by property name and optional value
- `LOGSEQ_DB_MODE=true` enables DB-mode property support
- Privacy: pages tagged with excluded tags are completely hidden from AI

**Direct file manipulation (common approach):**
- Because Logseq uses plain Markdown files, agents can read/write directly
- Properties are `key:: value` in markdown — grep/awk/sed friendly
- "Logseq is essentially a renderer for markdown files" — agents append blocks directly

**Plugins:**
- `logseq-plugin-ai-assistant` — GPT interaction from within Logseq
- `logseq-plugin-copilot` — Talk to AI about your notes
- No native AI agent SDK — all through plugins or external MCP

**Property discovery pattern:**
- Agent reads markdown files, parses `key:: value` lines
- Or uses `query` tool with Datalog: `[?p :block/properties ?props] [(get ?props :type) ?type]`
- No formal property type system — everything is string-based

### 4.3 AnyType — AI Agent Interaction

- No public REST API at time of research
- Local-first, P2P architecture — harder for external agents to integrate
- Community requests for API exist
- Internal architecture: everything is an Object with Properties (Relation objects) — conceptually agent-friendly if an API existed

---

## 5. Approaches for Quilt

### 5.1 Approach A: Notion-Style Layout Builder + Rich Property Types

Adopt Notion's three-zone layout builder (Pinned, Property Group, Panel) plus its full property type catalog.

**Pros:**
- Most users already know Notion's paradigm
- Proven UX — millions of users
- Rich property rendering (colored pills, date pickers, avatar chips, file thumbnails)
- Excellent API discoverability for AI agents

**Cons:**
- Complex UI to build (layout builder modal, drag-and-drop zones, collapsible sections)
- Heavy — Notion's approach requires a lot of frontend state
- Database-centric, not block-centric like Quilt

**Effort: High**

### 5.2 Approach B: AnyType-Style Type-Based Property Sets

Adopt AnyType's object-type-centric approach where properties are defined per type and reusable across types.

**Pros:**
- Cleaner separation of concerns — types define what properties exist
- Two-way linked properties are elegant (auto-sync backlinks into property slots)
- Four-zone visibility (Header/Panel/Hidden/Local) is simpler than Notion's builder
- Good fit for Quilt's domain model (already has Type concept)

**Cons:**
- Less visual richness than Notion
- Smaller property type catalog (9 types vs 22)
- Users may expect Notion's flexibility

**Effort: Medium**

### 5.3 Approach C: Logseq-Style Lightweight Key-Value + Datalog Queries

Keep properties as simple `key:: value` pairs at block level with powerful query capabilities.

**Pros:**
- Extremely simple to implement — just key-value parsing
- Plain text — easy for AI agents to read/write
- Datalog queries are powerful for property-based filtering
- Works naturally with Quilt's outliner model

**Cons:**
- No visual richness (no colored tags, no date pickers, no type validation)
- Users can't easily discover what properties exist
- No structured property types — everything is a string
- Feels primitive compared to Notion/AnyType

**Effort: Low**

### 5.4 Approach D: Hybrid — Type-Rich Properties + Block-Level Simplicity + Layout Zones (RECOMMENDED)

Combine the best of all three:
1. **Quilt's typed property system** (already exists) as the foundation
2. **Notion's layout zones**: Pinned (horizontal), Default (vertical with collapsible sections), Side Panel
3. **AnyType's type-based defaults**: each Quilt Type defines default visible properties and their zones
4. **Notion's rich rendering**: colored pills for selects, date pickers, avatar chips, file thumbnails
5. **Notion's API discoverability**: full property schema exposed via REST and MCP
6. **Logseq's simplicity for blocks**: block-level properties as lightweight key-value (optional, for quick annotations)
7. **Notion's relational model**: Relations and Rollups between pages/blocks

**Pros:**
- Rich UI where needed, simple where appropriate
- AI agents get full schema discoverability
- Users get visual property rendering
- Type-based defaults reduce setup friction
- Two-way linked properties from AnyType model

**Cons:**
- Most implementation effort
- Requires careful design to avoid inconsistency between page-level and block-level properties
- Need to decide: do blocks inherit page properties? How?

**Effort: High**

---

## 6. Specific Patterns Quilt Should Adopt

### 6.1 Property Type System

| Priority | Pattern | Source |
|----------|---------|--------|
| **P0** | Typed property system with validation per type | Notion API schema |
| **P0** | Select/Multi-select with colored options (10-color palette) | Notion |
| **P0** | Date property with date picker + range support | Notion |
| **P1** | Number formatting (currency, percent, progress bar) | Notion |
| **P1** | Checkbox as toggle (not text) | Notion |
| **P1** | File/Media with thumbnail previews | Notion |
| **P2** | Status type with predefined groups (To-do/In Progress/Done) | Notion |
| **P2** | Formula/Rollup for computed properties | Notion |
| **P2** | AI Autofill properties (summary, key info, translation) | Notion |
| **Future** | Two-way linked properties (auto-sync backlinks) | AnyType |

### 6.2 Property Layout

| Priority | Pattern | Source |
|----------|---------|--------|
| **P0** | Three-zone layout: Pinned (horizontal) + Default (vertical sections) + Side Panel | Notion |
| **P0** | Collapsible sections within property groups | Notion |
| **P0** | Property visibility toggle (hide/show per view) | Notion |
| **P1** | Type-level default property configuration | AnyType |
| **P1** | Per-object property overrides (hide specific properties) | Notion |
| **P2** | Multiple property groups with section labels | Notion |

### 6.3 Property Rendering

| Priority | Pattern | Source |
|----------|---------|--------|
| **P0** | Colored tag pills for Select/Multi-select | Notion |
| **P0** | Date picker popup (month grid) | Notion |
| **P0** | Clickable links for URL/Email/Phone types | Notion |
| **P1** | Avatar chips for Person type | Notion |
| **P1** | File thumbnail gallery | Notion |
| **P1** | Property descriptions as tooltips | Notion |
| **P2** | Conditional color on properties | Notion |

### 6.4 AI Agent Interface

| Priority | Pattern | Source |
|----------|---------|--------|
| **P0** | Full property schema exposed via REST API (`GET /types/{id} → properties`) | Notion API |
| **P0** | MCP tools for reading/writing properties with type validation | Notion MCP |
| **P0** | Property type discovery: agent can list all available properties and their types | Notion API |
| **P1** | AI autofill: define prompts that fill properties from content | Notion AI |
| **P1** | Query properties with filters ("find all pages where priority=high") | Notion/Legseq |
| **P2** | Agent validates property values before writing, returns specific validation errors | Notion API |

---

## 7. Key UI Flows to Implement (Concrete)

### 7.1 Add Property Flow
```
User clicks "+" next to last property → Modal opens:
  1. Text input: "Property name"
  2. Type selector grid: [Text] [Number] [Select] [Multi-select] [Date] [Checkbox] [URL] [Email] [Phone] [Person] [File] [Relation] [Status]
  3. For Select/Multi-select/Status: inline option editor appears (name + color for each option)
  4. "Save" button → property added to current view
```

### 7.2 Layout Builder Flow
```
User clicks "Customize layout" → Sidebar modal:
  ┌─ Layout Zones ──────────────────────┐
  │ ☰ Heading (Pinned)                  │
  │   [Property ▼] [+ Add property]     │
  │                                     │
  │ ☰ Property Group                    │
  │   [+ Add section]                   │
  │   Section: "Details"                │
  │     [Property ▼]                    │
  │     [Property ▼]                    │
  │                                     │
  │ ☰ Panel                             │
  │   [Property ▼] Style: [Large/Small] │
  │   [+ Add property]                  │
  └─────────────────────────────────────┘
  [Apply to all pages]  [Reset]
```

### 7.3 Property Edit Flow
```
Hover property → Click name → Dropdown:
  - Edit property
  - Duplicate
  - Delete
  - Hide
  - Insert left / Insert right
  
"Edit property" opens:
  - Rename
  - Change type (if compatible)
  - Add description (tooltip)
  - For Select: manage options (add, rename, recolor, delete, reorder via ⋮⋮)
```

---

## Risks

1. **Over-engineering**: Building a Notion-level layout builder is a massive UI undertaking. Start with simpler vertical layout + optional side panel.
2. **Type system rigidity**: If Quilt locks properties to types too strictly (like AnyType), users lose flexibility. Allow local/per-object property overrides.
3. **Block-level vs page-level confusion**: Logseq's distinction between page properties and block properties is subtle and confusing. Quilt should make this explicit: "Page properties" vs "Block properties" with clear UI boundaries.
4. **AI agent expectations**: Agents trained on Notion's API will expect property schemas in the Notion format. If Quilt deviates significantly, provide an adapter or clear documentation.
5. **Mobile**: Logseq's mobile property issues are a cautionary tale — test property editing on narrow viewports from day one.

---

## Ready for Proposal

**Yes.** The research is comprehensive. Recommended next step: create a design proposal (`sdd-design`) for a hybrid property system that combines Notion's layout zones + rich types, AnyType's type-based defaults, and Logseq's lightweight block-level key-value for quick annotations, with full API discoverability for AI agents.

## Key References
- [Notion Layouts Help](https://www.notion.com/help/layouts)
- [Notion Database Properties Help](https://www.notion.com/help/database-properties)
- [Notion API Property Object](https://developers.notion.com/reference/property-object)
- [Notion Layout Builder Deep Dive — Simone Smerilli](https://www.simonesmerilli.com/life/notion-layouts)
- [AnyType Properties Docs](https://doc.anytype.io/anytype-docs/getting-started/types/relations)
- [AnyType Templates Docs](https://doc.anytype.io/anytype-docs/getting-started/types/templates)
- [Logseq Properties Discussion](https://discuss.logseq.com/t/are-you-using-and-if-so-how-are-you-using-block-properties/20046)
- [Logseq Property Queries Tutorial](https://discuss.logseq.com/t/lesson-5-how-to-power-your-workflows-using-properties-and-dynamic-variables/10173)
- [Advanced Logseq Datalog Queries](https://www.eriksuniverse.com/advanced-logseq-queries-real-world.html)
- [Notion AI for Databases](https://www.notion.com/help/autofill)
- [mcp-logseq](https://skillsllm.com/skill/mcp-logseq)
- [Notion MCP vs API for AI Agents](https://www.scalekit.com/blog/notion-mcp-vs-api)
