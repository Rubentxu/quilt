# Default Templates

> ADR-0007: when a block carries `template:: <name>`, the frontend looks up the corresponding `template/<name>` page and applies its card-shape, icon, and CSS class.

In V1, Quilt ships with **two built-in templates** that the EmptyState buttons and the "quick add" buttons at the bottom of pages use by default. **You must create these template pages manually** for the buttons to work — they are NOT auto-seeded.

## The 2 built-in templates

### 1. `template/reference` — flat reference card

Activate a block with: `template:: reference`

Properties on the template page:

| Property | Value | Effect |
|----------|-------|--------|
| `card-shape::` | `reference` | Renders with icon, meta table, and open/copy actions |
| `icon::` | `🔗` | Shows next to the block's bullet |
| `cssclass::` | (optional) | CSS class applied to the card wrapper |

Recommended template page content:

```markdown
- template/reference                              ← the page name (no properties needed)
  card-shape:: reference                          ← tells CardRenderer which shape to use
  icon:: 🔗                                       ← visual decoration
```

When a block has `template:: reference`:

```
🔗 My linked reference                  [open] [copy]
   dda-relacionada: DDA v1
   type: Incidencia
   fecha-creacion: 26-05-2026
   ─────────────────────────────────────
   <the block content, editable>
```

### 2. `template/documentation` — collapsible content card

Activate a block with: `template:: documentation`

Properties on the template page:

| Property | Value | Effect |
|----------|-------|--------|
| `card-shape::` | `content` | Renders as collapsible card with header |
| `icon::` | `📄` | Shows in the card header |
| `cssclass::` | (optional) | CSS class applied to the card wrapper |

Recommended template page content:

```markdown
- template/documentation
  card-shape:: content
  icon:: 📄
```

When a block has `template:: documentation`:

```
▼ 📄 Pipelines documentation
   ─────────────────────────────────────
   <the block content, editable>
   - Pipeline A: ingest → ...
   - Pipeline B: validation → ...
```

The chevron collapses/expands the content.

## How to create them in Quilt

### Method 1: via the outliner

1. Create a page named `template/reference` (the `template/` prefix is what makes it a template)
2. On the first block, add properties:
   - `card-shape:: reference`
   - `icon:: 🔗`
3. Done. Now any block with `template:: reference` activates this card.

### Method 2: via API

```bash
# Create the page
curl -X POST http://localhost:3737/api/v1/pages \
  -H "Authorization: Bearer <API_KEY>" \
  -H "Content-Type: application/json" \
  -d '{"name":"template/reference"}'

# Add a block with the card-shape and icon
curl -X POST http://localhost:3737/api/v1/blocks \
  -H "Authorization: Bearer <API_KEY>" \
  -H "Content-Type: application/json" \
  -d '{
    "pageName": "template/reference",
    "content": "",
    "properties": [
      {"key":"card-shape","value":"reference","type":"string"},
      {"key":"icon","value":"🔗","type":"string"}
    ]
  }'
```

### Method 3: via the EmptyState buttons (self-bootstrapping)

In V1, the EmptyState buttons create blocks with `template:: reference` /
`template:: documentation` even if the template page doesn't exist yet. The
block is created with the property, but the card is not rendered until the
template page is created. The console will warn:

```
[getBlockCard] Block abc-123 references unknown template "reference". Falling back to inline.
```

Create the template page to silence the warning and activate the card.

## Create your own templates

The user can create **any number of templates** by following the same pattern:

1. Create a page `template/<your-template-name>`
2. Add the properties:
   - `card-shape:: reference` (flat card with metas + actions)
   - `card-shape:: content` (collapsible card)
   - `card-shape:: inline` (no card, just decoration)
3. Optionally:
   - `icon::` — emoji shown next to bullet (reference) or in header (content)
   - `cssclass::` — CSS class on the wrapper element

### Example: a "task" template

```markdown
- template/task
  card-shape:: inline
  icon:: ✅
  cssclass:: card-task
```

Then in `globals.css` or a custom CSS file:

```css
.card-task {
  border-left: 3px solid var(--color-primary);
  padding-left: 8px;
}
```

Now any block with `template:: task` gets the `card-task` class.

### Example: a "meeting notes" template

```markdown
- template/meeting-notes
  card-shape:: reference
  icon:: 📋
```

When a block has `template:: meeting-notes`, it renders as a reference card (the user fills in `attendees::`, `date::`, `agenda::` as metas inside the block).

## V2: agent-discoverable templates

In V2, MCP tools will let agents discover and apply templates without knowing the page names:

- `quilt_list_templates` — returns all templates with their card-shape, icon, and CSS class
- `quilt_get_template_schema` — returns the full property contract (required + optional properties)

For now, the V1 discoverability is the `api.listPages()` endpoint filtered by the `template/` prefix.

## See also

- `docs/adr/0007-template-driven-block-cards.md` — the design decision
- `docs/grill/implementation-plan.md` — the implementation roadmap
- `docs/wikilink-and-block-interactions.md` — the Logseq interaction research that informed the design
