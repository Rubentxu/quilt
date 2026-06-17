# Kernel Specs: GS-8 — Contextual Right Sidebar

> Source: `sddk/contextual-right-sidebar-gs8/proposal` (Engram #2277)
> Date: 2026-06-17

## Router Context Used
- **Knowledge Coverage:** sufficient — proposal resolved the ADR-0020↔0030 conflict (Option B: narrowed, not rescinded), defined SelectionContext contract, RightSidebarSection registry, and deterministic ranker heuristic. Exploration confirmed PanelVisibilityContext machinery is reusable.
- **Context Quality:** C2 — durable ADRs exist (0030 §14, 0020), migration plan Phase 8 is on file, reversa confirms the Logseq pattern. The C2→C3 gap is the missing implementation (no global selection state, no unified sidebar shell, no E2E coverage for the unified surface).
- **Taxonomy:** domain-modeling (new SelectionContext concept), boundary/seam (two independent columns → one unified shell), coupling/connascence (6+ consumers of selection state), refactor/legacy (BacklinksPanel + CognitivePanels migrate into sections, BlockPropertiesPanel mounts in sidebar), testing (new E2E spec needed)
- **Domain Language (resolved):** panel derecho contextual, selección activa, resolvedor de contexto, acción principal, contexto de página/journal, contexto general del Graph, panel-first, preferencia global de visibilidad
- **Domain Language (ambiguous):** None — proposal resolved: "selección activa" includes page-level fallback when no block is selected; "acción principal con alta confianza" is a deterministic v1 heuristic (no server roundtrip); "edición rica de properties" uses the existing BlockPropertiesPanel mounted in the sidebar.
- **Recommended Effort:** verify — the proposal already resolved the critical open questions; specs now codify the contracts.

## Knowledge Provenance
- **Scope source:** `sddk/contextual-right-sidebar-gs8/proposal` (Engram #2277), `sddk/contextual-right-sidebar-gs8/explore.md` (this directory)
- **Invariant source:** `docs/adr/0030-graph-space-journal-first-lifecycle.md` §14; `docs/adr/0020-property-editing-surface-fixed-header.md` (narrowed by proposal); `docs/graph-space-migration-plan.md` Phase 8; `docs/quilt-keyboard-shortcuts.md` line 145
- **Memory-only hints excluded from spec truth:**
  - The proposal's internal "Learned" field (ADR-0020 narrowed, ADR-0020 block-focus-populates-header superseded) is now durably recorded in this spec and will be recorded in ADR-0031 when created during design phase.
  - The specific heuristic weights for the main-action ranker are a design-phase detail; the spec constrains only the contract (0 or 1 action, confidence threshold, deterministic).

---

## Capability: selection-context

### Requirement: Global active-selection state
The system SHALL maintain a single global `SelectionContext` that exposes the currently active selection to all sidebar consumers. The context SHALL hold a `Selection` value object that is one of: `BlockSelection { blockId: string, pageName: string }`, `PageSelection { pageName: string }`, `JournalSelection { date: string }`, `GraphSelection`, or `None`.

The selection SHALL respect the priority order defined in ADR-0030 §14:
1. Active block selection (when the user has focused or clicked a block)
2. Page/journal context (derived from the current route when no block is selected)
3. Graph context (fallback for routes without a page/journal backing: `/graph`, `/pages`, `/settings`, `/`)

The `SelectionContext` SHALL be ephemeral — it is NOT persisted to `localStorage` across sessions. Selection state SHALL be lost on page reload.

#### Scenario: Block selection populates the context
**Given** the user is on route `/page/Research` and no block is selected
**When** the user clicks on block `abc-123` inside the page
**Then** the `SelectionContext` holds `BlockSelection { blockId: "abc-123", pageName: "Research" }`
**And** all sidebar sections that consume `useSelection()` receive the block selection

#### Scenario: Page-level fallback when no block is selected
**Given** the user is on route `/page/Research` and no block is selected
**When** the page renders
**Then** the `SelectionContext` holds `PageSelection { pageName: "Research" }`

#### Scenario: Journal fallback when no block is selected
**Given** the user is on route `/journal/2026-06-17` and no block is selected
**When** the journal page renders
**Then** the `SelectionContext` holds `JournalSelection { date: "2026-06-17" }`

#### Scenario: Graph-level fallback for non-page routes
**Given** the user navigates to `/graph`, `/pages`, `/settings`, or `/`
**When** the route renders
**Then** the `SelectionContext` holds `GraphSelection`

#### Scenario: Clear selection on route change
**Given** the `SelectionContext` holds `BlockSelection { blockId: "abc-123", pageName: "Research" }`
**When** the user navigates to `/journal/2026-06-17` (a different route)
**Then** the `SelectionContext` transitions to `JournalSelection { date: "2026-06-17" }`
**And** no stale block selection is retained from the previous route

#### Scenario: Clear block selection on ESC key
**Given** the `SelectionContext` holds `BlockSelection { blockId: "abc-123", pageName: "Research" }`
**When** the user presses the `Escape` key
**Then** the `SelectionContext` transitions to `PageSelection { pageName: "Research" }` (the fallback for the current route)
**And** the block's visual focus indicator is removed

#### Scenario: Selection persists within the same route
**Given** the `SelectionContext` holds `BlockSelection { blockId: "abc-123", pageName: "Research" }`
**When** the user scrolls within `/page/Research` without changing routes
**Then** the selection remains `BlockSelection { blockId: "abc-123", pageName: "Research" }`

#### Scenario: SelectionContext is empty on initial app load (before hydration)
**Given** the app has just loaded and no route has resolved
**When** the `SelectionContext` is read before any page has rendered
**Then** it holds `None`

---

## Capability: right-sidebar-section-registry

### Requirement: Dynamic section registration
The system SHALL provide a `RightSidebarSectionRegistry` that allows any feature module to register a sidebar section. Each registration SHALL include: a unique `sectionId`, a `priority` integer (lower numbers render first), a `render` function returning a React node, and a `predicate` function that receives the current `Selection` and returns `boolean` (whether the section should render).

The registry SHALL be a singleton module (not React context) so features can register at module load time without a React provider wrapper.

#### Scenario: Section registers at module load time
**Given** the `right-sidebar/sections` module imports the backlinks feature
**When** the backlinks feature calls `rightSidebarSections.register({ sectionId: "backlinks", priority: 200, predicate: (s) => s.type === "page" || s.type === "journal", render: () => <BacklinksSection /> })`
**Then** the registry contains the `backlinks` section
**And** subsequent calls to `getSectionsForSelection(selection)` include it when the predicate matches

#### Scenario: Sections are ordered by ascending priority
**Given** three sections registered with priorities 100 ("properties"), 200 ("backlinks"), and 300 ("cognitive-overview")
**When** `getSectionsForSelection(selection)` is called
**Then** the returned array is ordered `[properties, backlinks, cognitive-overview]`
**And** this order is stable across calls

#### Scenario: Sections with equal priority are ordered by registration time
**Given** section A (priority 100) registers first, then section B (priority 100) registers second
**When** `getSectionsForSelection(selection)` is called
**Then** section A appears before section B in the result

#### Scenario: Section is hidden when predicate returns false
**Given** a section registered with `predicate: (s) => s.type === "block"` (only shows for block selections)
**When** the current selection is `PageSelection { pageName: "Research" }`
**Then** `getSectionsForSelection(selection)` does NOT include this section

#### Scenario: Section render receives the current selection
**Given** a section registered with `render: (selection) => <MySection selection={selection} />`
**When** the section is rendered
**Then** the `render` function receives the current `Selection` value as its argument

---

## Capability: main-action-ranker

### Requirement: Deterministic action ranking
The system SHALL evaluate all registered sidebar sections' declared actions and select at most one "main action" to display in the sidebar's top slot. The ranker SHALL be a pure function: given the current `Selection` and an `Action[]` list from registered sections, it SHALL return `{ action: Action, confidence: number } | null`.

The ranker SHALL apply these deterministic v1 rules:
1. If no section declares any actions for the current selection, return `null`
2. If exactly one action is declared, return it with confidence `1.0` if it comes from a section whose area matches the selection type (e.g., properties section for `BlockSelection`), otherwise confidence `0.5`
3. If multiple actions are declared, select the one with the highest priority section (lowest section priority number); if tied, pick the first registered
4. Actions labeled as `kind: "suggestion"` from cognitive panels receive a confidence penalty of `-0.3` compared to `kind: "action"` from structural sections
5. Return the action only if its computed confidence ≥ `0.7`; otherwise return `null`

The ranker SHALL NOT make any network requests or async calls. It SHALL be fully deterministic (same inputs → same output).

#### Scenario: No actions — returns null
**Given** no registered section declares any action for the current `PageSelection`
**When** the ranker evaluates
**Then** it returns `null`
**And** the sidebar's main-action slot is empty

#### Scenario: Single action with matching selection type — high confidence
**Given** the properties section declares `{ id: "edit-properties", label: "Edit Properties", kind: "action" }` for `BlockSelection`
**When** the selection is `BlockSelection` and no other sections declare actions
**Then** the ranker returns `{ action: { id: "edit-properties", ... }, confidence: 1.0 }`

#### Scenario: Single action from mismatched selection type — lower confidence
**Given** the cognitive-overview section declares `{ id: "review-connections", kind: "suggestion" }` for `GraphSelection`
**When** the selection is `BlockSelection` (mismatched)
**Then** the action is evaluated with confidence `0.5 - 0.3 = 0.2` (below threshold)
**And** the ranker returns `null`

#### Scenario: Multiple actions — highest-priority section wins
**Given** the backlinks section (priority 200) declares `{ id: "show-pages" }`, and the properties section (priority 100) declares `{ id: "edit-properties" }`
**When** both predicates match the current `BlockSelection`
**Then** the ranker selects `{ id: "edit-properties" }` (from the priority-100 section), with confidence derived from the matching selection type
**And** the lower-priority action is ignored

#### Scenario: Suggestion with high base confidence still passes threshold
**Given** only the cognitive-overview section (kind `"suggestion"`) declares an action for `BlockSelection` where the section predicate matches the selection type
**When** the ranker evaluates
**Then** confidence = `1.0 - 0.3 = 0.7` (meets threshold)
**And** the action is returned

#### Scenario: Action confidence below threshold — returns null
**Given** a single action from a mismatched section with kind `"suggestion"`
**When** confidence computes to `0.5 - 0.3 = 0.2`
**Then** `0.2 < 0.7`, so the ranker returns `null`
**And** the main-action slot is empty

---

## Capability: unified-sidebar-shell

### Requirement: Single contextual right column
The system SHALL render a single unified right sidebar column (`<aside>`) in the AppShell, replacing the current two-column layout (independent `BacklinksPanel` + `CognitivePanels` columns). The column SHALL be 320px wide with `min-width: 320px` and SHALL have `flex-shrink: 0`.

The shell SHALL consume `SelectionContext` to derive the active selection, then call `getSectionsForSelection(selection)` from the section registry to determine which sections to render.

The shell SHALL call the main-action ranker with the current selection and the actions declared by visible sections, and SHALL render the main action in a dedicated slot at the top of the sidebar when the ranker returns a non-null result.

On mobile (viewport width ≤ 768px), the sidebar SHALL render as a bottom sheet with a backdrop, matching the existing pattern in `AppShell.tsx:893-926` for the current mobile BacklinksPanel.

#### Scenario: Sidebar visible on desktop by default
**Given** a desktop viewport (width > 768px) and the app has loaded with the default preset
**When** the AppShell renders
**Then** the right sidebar `<aside>` is visible (not `display: none`)
**And** the sidebar occupies 320px of width on the right side of the viewport

#### Scenario: Sidebar hidden when toggled off
**Given** the right sidebar is visible
**When** the user clicks the collapse/close button on the sidebar
**Then** the sidebar slides/hides off-screen (or `display: none`)
**And** the main content area expands to fill the reclaimed width

#### Scenario: Sidebar shown when toggled on
**Given** the right sidebar is hidden
**When** the user clicks the toggle button in the top bar or presses `t r`
**Then** the sidebar reappears
**And** the main content area contracts to accommodate the 320px column

#### Scenario: Visibility persisted to localStorage
**Given** the user hides the right sidebar
**When** the page is reloaded
**Then** the sidebar is still hidden on load
**And** the persisted visibility preference is read from `quilt-dashboard-layout` in `localStorage`

#### Scenario: Default preset includes right sidebar
**Given** a fresh install (no prior `localStorage` value)
**When** the app loads
**Then** the `right-sidebar` panel is in the `default` preset (visible)
**And** the `PanelVisibilityContext` `visiblePanels` set includes `"right-sidebar"`

#### Scenario: Sidebar header renders section tabs
**Given** three sections match the current selection: properties (priority 100), backlinks (priority 200), cognitive-overview (priority 300)
**When** the sidebar renders
**Then** the sidebar header shows three section-tab labels in priority order
**And** the highest-priority section (properties) is expanded by default
**And** clicking a different tab label scrolls to or expands that section

#### Scenario: Main action slot renders when ranker returns an action
**Given** the main-action ranker returns `{ action: { id: "edit-properties", label: "Edit Properties" }, confidence: 1.0 }`
**When** the sidebar shell renders
**Then** the main-action slot at the top of the sidebar displays a clickable button labeled "Edit Properties"
**And** clicking the button executes the action's `onExecute` callback

#### Scenario: Main action slot empty when ranker returns null
**Given** the main-action ranker returns `null`
**When** the sidebar shell renders
**Then** the main-action slot is not rendered (no dead space)

#### Scenario: Mobile bottom sheet
**Given** a mobile viewport (width ≤ 768px)
**When** the right sidebar is toggled open
**Then** the sidebar renders as a bottom sheet anchored to the bottom of the viewport
**And** a semi-transparent backdrop appears behind it
**And** tapping the backdrop dismisses the bottom sheet
**And** the bottom sheet has `max-height: 60vh` and rounded top corners

---

## Capability: property-editing-split

### Requirement: Page-level properties in header, block-level in sidebar
The system SHALL render page-level properties in the fixed header section (below the page title, per ADR-0020). The system SHALL render block-level properties in the right sidebar (per ADR-0030 §14) when a block is actively selected.

The page-level header SHALL show properties for the current page regardless of whether a block is selected. The header SHALL NOT change its content when a block is selected (the ADR-0020 "block-focus-populates-header" behavior is superseded).

#### Scenario: Page-level properties shown in header on page load
**Given** the user navigates to `/page/Research`, which has page-level properties `{ status: "active", priority: "high" }`
**When** the page renders and no block is selected
**Then** the fixed header section displays the page-level properties `status` and `priority`
**And** the right sidebar's properties section shows no block-level properties (or shows the page-level properties in read-only mode as a fallback)

#### Scenario: Block selection shows block properties in sidebar
**Given** the user is on `/page/Research` and clicks on block `abc-123`, which has properties `{ assignee: "Alice", due: "2026-07-01" }`
**When** the `SelectionContext` updates to `BlockSelection { blockId: "abc-123", pageName: "Research" }`
**Then** the right sidebar's properties section mounts `BlockPropertiesPanel` with `blockId = "abc-123"` as the primary editing surface
**And** the fixed header continues to show the page-level properties `status` and `priority` (unchanged)

#### Scenario: Header does NOT change on block selection
**Given** the user is on `/page/Research` and the header shows page-level properties `{ status: "active" }`
**When** the user selects block `abc-123` (which has properties `{ assignee: "Alice" }`)
**Then** the header STILL shows `{ status: "active" }` (NOT replaced by `{ assignee: "Alice" }`)
**And** the block properties only appear in the right sidebar

#### Scenario: Deselecting a block reverts sidebar properties to page-level
**Given** a `BlockSelection` is active and the sidebar shows block properties
**When** the user presses `Escape` (clearing the selection to `PageSelection`)
**Then** the sidebar properties section transitions from block-level to page-level properties display
**And** the page-level properties are read-only in the sidebar (the authoritative page-level property editor remains the fixed header)

#### Scenario: Block properties edited in sidebar persist correctly
**Given** the sidebar properties section renders `BlockPropertiesPanel` for block `abc-123`
**When** the user changes the `assignee` property from "Alice" to "Bob" via the sidebar panel
**Then** the change is persisted to the server
**And** the `InlinePropertyBadges` in the block row reflect "Bob"
**And** a second block selection (block `def-456`) shows its own properties (not the stale `assignee = "Bob"` from the previous block)

#### Scenario: Inline BlockPropertiesPanel in BlockRow is NOT removed
**Given** the existing `BlockPropertiesPanel` rendered inside `BlockRow.tsx` (triggered by the Settings2 button or context menu)
**When** GS-8 is deployed
**Then** the inline panel remains functional as a quick-edit alternative
**And** both the inline panel and the sidebar panel can be open simultaneously
**And** changes from either surface are reflected in the other (they share the same data source)

---

## Capability: section-migration

### Requirement: BacklinksPanel migrates into the sidebar as a section
The existing `BacklinksPanel` SHALL be registered as a section in the `RightSidebarSectionRegistry` with `sectionId = "backlinks"` and `priority = 200`. It SHALL be rendered inside the unified right sidebar shell, NOT as a separate `<aside>` column in `AppShell.tsx`.

The backlinks section SHALL receive the current selection from `SelectionContext` and SHALL resolve the page name from `PageSelection.pageName`, `JournalSelection.date` (derived page for that date), or `BlockSelection.pageName`. It SHALL NOT render when the selection is `GraphSelection` or `None`.

#### Scenario: Backlinks section renders for page selection
**Given** the current selection is `PageSelection { pageName: "Research" }`
**When** the sidebar renders
**Then** the backlinks section is visible (the predicate matches)
**And** `BacklinksPanel` receives `pageName = "Research"`

#### Scenario: Backlinks section renders for block selection (resolved to the block's page)
**Given** the current selection is `BlockSelection { blockId: "abc-123", pageName: "Research" }`
**When** the sidebar renders
**Then** the backlinks section resolves the page name as `"Research"`
**And** the section renders backlinks for the page `"Research"`

#### Scenario: Backlinks section hidden for GraphSelection
**Given** the current selection is `GraphSelection`
**When** the sidebar renders
**Then** the backlinks section is NOT included in the visible sections

#### Scenario: BacklinksPanel is no longer a standalone column
**Given** the GS-8 deployment
**When** the AppShell renders
**Then** there is no separate `<aside>` column specifically for `BacklinksPanel`
**And** the backlinks content is only rendered inside the unified right sidebar shell

### Requirement: Cognitive panels migrate into the sidebar as sections
Each cognitive panel that was previously gated by `PanelVisibilityContext` flags in `CognitivePanels.tsx` SHALL be registered as an individual section in the `RightSidebarSectionRegistry`. The cognitive panels SHALL include: `agent-activity` (priority 300), `agent-room` (priority 310), `structural-graph` (priority 320), `semantic-insight` (priority 330), `cognitive-graph` (priority 340), `decay-monitor` (priority 350), `weekly-review` (priority 360), `serendipity` (priority 370).

Each cognitive section SHALL gate its visibility via both the existing `PanelVisibilityContext` flag AND its registry predicate. The `CognitivePanels.tsx` wrapper component SHALL be retired as a standalone column in `AppShell.tsx`.

#### Scenario: Agent-activity section renders when its panel flag is set
**Given** the `agent-activity` panel flag is set in `PanelVisibilityContext`
**When** the current selection is `GraphSelection`
**Then** the agent-activity section is visible in the right sidebar
**And** it renders the same `AgentActivityFeed` content it previously rendered in `CognitivePanels.tsx`

#### Scenario: Cognitive section hidden when its panel flag is off
**Given** the `weekly-review` panel flag is NOT set in `PanelVisibilityContext`
**When** the sidebar renders
**Then** the weekly-review section is NOT visible, regardless of the current selection

#### Scenario: CognitivePanels.tsx wrapper is retired
**Given** the GS-8 deployment
**When** the AppShell renders
**Then** there is no standalone `<CognitivePanels>` component rendered as a separate `<aside>` column
**And** all cognitive panel content is only rendered inside the unified right sidebar shell via the section registry

---

## Capability: empty-state

### Requirement: Graph-level empty state when no selection is active
When the sidebar renders with a `GraphSelection` or `None` selection, and no cognitive panels are enabled (or all return null content), the system SHALL display a compact empty state instead of a blank 320px column.

The empty state SHALL display:
- A brief label: "Right Sidebar" or icon
- The text: "Select a block to see its properties, backlinks, and actions here."
- A subtle link: "Open keyboard shortcuts" that links to the shortcuts help

The empty state SHALL NOT trigger any network requests. It SHALL be static content.

#### Scenario: Empty state when no selection and no cognitive panels enabled
**Given** the user is on route `/` (home), no block is selected, the selection is `None`
**And** no cognitive panels have their flags enabled in `PanelVisibilityContext`
**When** the sidebar renders
**Then** it displays the empty state with the instructional text
**And** no blank 320px space is shown

#### Scenario: Empty state suppressed when cognitive panels render content
**Given** the selection is `GraphSelection` and the `agent-activity` panel is visible and returns content
**When** the sidebar renders
**Then** the empty state is NOT shown
**And** the agent-activity section content is displayed instead

#### Scenario: Empty state transitions to content when selection changes
**Given** the sidebar shows the empty state (no selection)
**When** the user clicks on a block, setting `BlockSelection`
**Then** the empty state is immediately replaced by the properties and backlinks sections
**And** there is no flash of the empty state during the transition

---

## Capability: keyboard-shortcut-t-r

### Requirement: `t r` toggles the right sidebar
The system SHALL implement the keyboard shortcut `t r` (leader key `t`, then `r`) to toggle the right sidebar visibility. This SHALL match the documented shortcut in `docs/quilt-keyboard-shortcuts.md` line 145.

The shortcut SHALL extend the existing leader-key system (`useGlobalShortcuts` in `AppShell.tsx`) to accept `t` as a leader key (in addition to the existing `g` leader key). When `t` is pressed and not inside a content-editable element, the system SHALL enter `t`-leader mode for 1.5 seconds, during which pressing `r` toggles the right sidebar's visibility.

The toggle SHALL dispatch the same `dispatchDashboardChange({ type: 'toggle', panel: 'right-sidebar' })` custom event used by the command palette's `layout/toggle-*` commands.

#### Scenario: Press `t` then `r` hides the visible sidebar
**Given** the right sidebar is visible
**And** focus is not inside a `contentEditable`, `<input>`, or `<textarea>`
**When** the user presses `t` (releasing it) and then presses `r` within 1.5 seconds
**Then** the right sidebar hides
**And** the leader key mode is dismissed

#### Scenario: Press `t` then `r` shows the hidden sidebar
**Given** the right sidebar is hidden
**And** focus is not inside an editable element
**When** the user presses `t` then `r`
**Then** the right sidebar becomes visible

#### Scenario: Leader key `t` times out without second key
**Given** the right sidebar is visible
**When** the user presses `t` and waits 1.5+ seconds without pressing any other key
**Then** the leader key mode is dismissed
**And** the sidebar remains visible (no toggle)

#### Scenario: `t r` ignored when focus is in a content-editable element
**Given** focus is inside a `contentEditable="true"` block
**When** the user presses `t` then `r`
**Then** the characters `t` and `r` are typed into the block as normal text
**And** the leader key system does NOT intercept the keys
**And** the sidebar visibility does NOT change

#### Scenario: `t` followed by unknown key is a no-op
**Given** the right sidebar is visible and the user has pressed `t` (leader mode active)
**When** the user presses `z` within 1.5 seconds
**Then** the leader mode is dismissed
**And** the sidebar remains visible (no toggle, no error)

#### Scenario: `t r` registered as a command in the command palette
**Given** the command center is open
**When** the user searches for "toggle right sidebar"
**Then** a command `layout/toggle-right-sidebar` appears in the results
**And** executing it has the same effect as pressing `t r`

---

## Invariants Covered

| Invariant (source) | Coverage |
|-----------|----------|
| Right sidebar visible by default in desktop (ADR-0030 §14) | `unified-sidebar-shell: Sidebar visible on desktop by default`, `unified-sidebar-shell: Default preset includes right sidebar` |
| Right sidebar is collapsible/hideable (ADR-0030 §14) | `unified-sidebar-shell: Sidebar hidden when toggled off`, `unified-sidebar-shell: Sidebar shown when toggled on`, `keyboard-shortcut-t-r: Press t then r hides/shows` |
| Visibility persisted as global preference (ADR-0030 §14) | `unified-sidebar-shell: Visibility persisted to localStorage` |
| Priority: selection > page/journal > graph (ADR-0030 §14) | `selection-context: Block selection populates the context`, `selection-context: Page-level fallback`, `selection-context: Graph-level fallback` — the priority order is encoded in the SelectionContext resolver |
| Property editing is panel-first (primary surface) (ADR-0030 §14) | `property-editing-split: Block selection shows block properties in sidebar` — the sidebar is the primary surface for block-level properties |
| ≤ 1 main action, only when confidence high (ADR-0030 §14) | `main-action-ranker` all scenarios — the ranker returns 0 or 1 action, confidence threshold enforced at 0.7 |
| ADR-0020 fixed header keeps page-level properties; block-focus-header interaction superseded | `property-editing-split: Header does NOT change on block selection` |
| SelectionContext ephemeral (not persisted across sessions) | `selection-context` — explicit requirement: "NOT persisted to localStorage" |
| BacklinksPanel + CognitivePanels migrate into unified shell as sections | `section-migration` all scenarios |
| `t r` keyboard shortcut matches Logseq parity | `keyboard-shortcut-t-r` all scenarios |
| Mobile: bottom sheet pattern matching existing AppShell.tsx precedent | `unified-sidebar-shell: Mobile bottom sheet` |

## Open Questions

- **ADR-0031 creation**: The proposal states the ADR-0020↔0030 reconciliation will be recorded as ADR-0031. This ADR does not yet exist. The design phase should create it, documenting the narrowed scope of ADR-0020 and the superseded block-focus→header interaction.
- **Inline BlockPropertiesPanel deprecation timeline**: The inline panel in `BlockRow.tsx` is kept as a quick-edit alternative per the proposal. No timeline is set for its eventual deprecation. The spec leaves both surfaces alive; a follow-up change can deprecate the inline panel when the sidebar proves sufficient.
- **Section tab header UX**: The spec says the highest-priority section is "expanded by default." Whether the sidebar renders sections as collapsible accordions (one expanded at a time) or a vertical stack (all visible, scrollable) is a design-phase UX decision. The spec constrains only order and visibility.
- **Mobile cognitive panels**: The current `CognitivePanels` column is hidden on mobile (`!isMobile` guard in AppShell.tsx:944). The spec's mobile bottom sheet covers the right sidebar shell but the treatment of individual cognitive sections on mobile (show all? show only agent-activity?) is deferred to design.
- **Leader key `t` conflicts**: Adding `t` as a second leader key may conflict with future shortcuts starting with `t` (e.g., `t l`, `t d`, `t b` from `docs/quilt-keyboard-shortcuts.md`). The spec only defines `t r`; implementing the full `t *` family should extend the same leader-key handler without duplicating the state machine.
