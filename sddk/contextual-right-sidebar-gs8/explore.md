# Kernel Exploration: GS-8 — Contextual right sidebar (visible by default, collapsible, selection-prioritized, panel-first for properties)

> Source: `sddk/contextual-right-sidebar-gs8/explore`
> Date: 2026-06-17
> Topic: Phase 8 of `docs/graph-space-migration-plan.md` and ADR-0030 §14
> Status: **explore — ready for proposal**

---

## Current State

### What GS-8 must deliver (per ADR-0030 §14 and Phase 8 of the migration plan)

ADR-0030 §14 states (verbatim, ES):

> "En desktop, el panel derecho:
> - está visible por defecto
> - es colapsable/ocultable
> - recuerda su visibilidad como preferencia global
> Su prioridad contextual es:
> 1. selección activa
> 2. contexto de página/journal
> 3. contexto general del Graph
> Es la superficie principal para:
> - edición rica de properties
> - metadata
> - acciones contextuales
> - sugerencias
> - contexto semántico
> Puede mostrar como máximo **0 o 1 acción principal**, solo cuando la confianza contextual sea alta."

`docs/graph-space-migration-plan.md` Phase 8 expands this into a technical contract:

> "Pasar de inspector accesorio a superficie operativa contextual.
> Contrato inicial:
> - visible por defecto en desktop
> - colapsable/ocultable
> - prioridad por selección activa
> - edición de properties panel-first
> - una acción principal máximo con alta confianza
> Necesidades técnicas:
> - modelo de selección activa
> - resolvedor de contexto
> - ranking de acciones
> - ordenamiento dinámico de secciones"

So GS-8 must turn the current "backlinks + cognitive columns" into one **unified, contextual, selection-aware** right sidebar. The pieces are partially in place; the integration is not.

### What the right side of the shell renders today

`quilt-ui/src/shared/components/AppShell.tsx` composes the right side as **two independent `<aside>` columns** rendered after the main `<Outlet />`:

1. **`<BacklinksPanel pageName={currentPageName} isOpen={backlinksOpen} />`** — `AppShell.tsx:928`
   - Width 320px, `borderLeft`, `flex-shrink: 0`
   - `currentPageName` is derived from URL pathname (`deriveCurrentPageName`, `AppShell.tsx:66-87`) — *page-level only, no block-level selection*
   - `isOpen` driven by `visiblePanels.has('backlinks')` in `PanelVisibilityContext`
   - **Default preset includes `backlinks`** (`features/dashboard/presets.ts:55`) so it is visible by default
   - Inner content is collapsed by default (`defaultExpanded = false` in `BacklinksPanel.tsx:24`) — user must click the header to expand the list
   - The "Linked References" header shows just a count badge; no properties, no actions

2. **`<CognitivePanels pageName={currentPageName} />`** — `AppShell.tsx:944`
   - Width 320px, `borderLeft`, `flex-shrink: 0`
   - **Hidden by default** — returns `null` unless at least one of the eight panel flags is set (`CognitivePanels.tsx:53-64`)
   - Panels it can host: `agent-activity`, `agent-room`, `structural-graph`, `semantic-insight`, `cognitive-graph`, `decay-monitor`, `weekly-review`, `serendipity`
   - Each panel is a self-contained section; no shared contract, no selection awareness, no priority resolver

So the "right side" today is a **320 + 320 = 640px** chrome column (when both visible) split into two unrelated features, neither of which is selection-aware at the block level.

### How "selection" works today (and why it does not satisfy §14)

The codebase has **no global active-selection state**:

- `BlockContext.tsx` (`features/outliner-tiptap/BlockContext.tsx:5-18`) is a **per-row** React context. Its `block`, `allBlocks`, `pageName` are all scoped to one BlockRow. Sibling rows do not see each other.
- `onFocusBlock` (`PageView.tsx:985-1026`) focuses a `contentEditable` for keyboard navigation. It is cursor-focus, not selection-state, and is local to PageView.
- `BacklinksPanel` reads the page name from the **URL pathname** — there is no per-block selection being read.
- `BlockRow.tsx:236` has `const [showProperties, setShowProperties] = useState(false)` — a per-row UI toggle, not a selection model.

The right sidebar's only signal of "what the user is looking at" is the route — `/page/<name>` or `/journal/<date>`. ADR-0030 §14's priority 1 ("active selection") has no implementation surface to read from yet. This is the most critical missing piece for GS-8.

### Where properties are edited today

Properties are edited **inline, below the block, inside BlockRow**:

- `<BlockPropertiesPanel blockId={block.id} onClose={...} />` — `BlockRow.tsx:1702`
- Rendered conditionally on `showProperties` (`BlockRow.tsx:1693-1708`)
- Two triggers:
  - Hover-revealed `Settings2` button on the row (`BlockRow.tsx:1625`)
  - `BlockContextMenu` "Properties" item (`BlockContextMenu.tsx:156-168`) — right-click → menu item → opens the inline panel
- The panel itself (`features/properties/BlockPropertiesPanel.tsx:114`) supports rich editing: typed inputs (string/number/boolean), NL date resolution, derived/immutability guards, add/delete property, system property toggle.
- Properties are also surfaced as `InlinePropertyBadges` and `PropertyStrip` *inside* the block content (`BlockRow.tsx:1388, 1501`), per ADR-0020 (Fixed Header model — see below).

**There is no "right sidebar property editor" today.** Properties are a block-local concern rendered inline.

### Existing actions / suggestions

- **Block-level** (in-row context menu, `BlockContextMenu.tsx:116-188`): add-child, move-up, move-down, convert-to-task, properties, copy-link, delete. Always available when the row's context menu is open.
- **Cognitive suggestions** (gated by panel flags):
  - `WeeklyReview.tsx:417-492` — `data.suggestions[]` with focus summary
  - `Serendipity.tsx:267-280` — accept/ignore on suggested connections
  - `SemanticInsight.tsx`, `StructuralGraph.tsx`, `DecayMonitor.tsx` — insight-style content
  - `AgentActivityFeed.tsx` — last 15 agent-authored blocks
- **Command palette** (`features/command-center/builtin-commands.ts`): 16 built-in commands (5 nav, 2 view, 7 layout, 2 cog, 1 capture, 1 help). Pure navigation/visibility — no "contextual action" concept.
- **No "main action with high confidence" pattern exists.** The BlockContextMenu shows up to 7 actions, the cognitive panels show whatever their heuristic surfaces, the command palette lists everything.

### Tabs as the existing contextual surface (and why GS-8 differs)

`shared/contexts/TabsContext.tsx` provides a tab bar (route-level) with `openTab({ type, name, params })`. Tabs are **route-level** (one per page/journal/graph/settings/all-pages), not block-level. They are not the contextual right sidebar ADR-0030 §14 describes; they are an orthogonal model.

### What ADR-0020 says vs what GS-8 asks for (CONTRADICTION TO SURFACE)

**ADR-0020 — Property Editing Surface: Fixed Header** (`docs/adr/0020-property-editing-surface-fixed-header.md`, accepted 2026-06-03) explicitly rejected the right sidebar for properties:

> "❌ Right sidebar for v1 — deferred complexity, two-surface editing model"

The decision was a "Fixed always-visible property header section below page title" with block focus populating the same header. Reasoning included CRDT simplicity (2 states vs 3^N visibility states), single editing surface, AI-first machine-queryable position, and mobile consistency.

ADR-0030 §14 (accepted 2026-06-17) now mandates the right sidebar as the **primary** property editing surface — a direct conflict with ADR-0020's "block focus populates the same header" model. The conflict is not addressed in either ADR's "related" section. The proposal phase must surface this and either:

- (a) rescind ADR-0020 explicitly, or
- (b) define the boundary: header shows page-level properties at all times; right sidebar shows the *active-selection's* properties (block-level when a block is selected, page-level when nothing is selected). Both exist, with clear scope.

This is the most important architectural question for GS-8.

### Logseq reversa (what the right sidebar *is*, by precedent)

`docs/reversa/logseq-ui-reference.md:56-67, 327, 341, 400, 630` and `docs/reversa/_reversa_sdd/frontend-components.md:228-477` describe Logseq's right sidebar (`cp__right-sidebar`, `right_sidebar.cljs`, 528 lines):

- Single contextual column, min 320px / max 70% viewport (`components:144`, `right_sidebar.cljs:352-354`)
- Hosts contextual panels + TOC + page contents
- Shift+Enter / Shift+Click → "open in right sidebar" (`logseq-ui-reference.md:327, 341, 400`)
- Keyboard shortcut `t r` to toggle
- TOC inside the right sidebar
- Wider concept than Quilt's current "two independent columns"

The reversa analysis (`confidence-report.md:43-44`) confirms these width constraints. This is the design target GS-8 is reaching for.

### Existing presets and panel visibility machinery

`features/dashboard/PanelVisibilityContext.tsx` already implements:

- `Set<PanelId>` of currently visible panels
- `localStorage` persistence under `DASHBOARD_STORAGE_KEY = 'quilt-dashboard-layout'`
- Custom DOM event for non-React callers (CommandRegistry dispatches `quilt:dashboard-layout-change`)
- Preset matching (`default`, `focus`, `review`)
- `PANEL_LABELS` — single source of truth for human-readable labels

This is exactly the machinery GS-8 needs for "visible por defecto, colapsable, recuerda su visibilidad como preferencia global" — the only thing missing is the *content* of that sidebar (the contextual resolver, the property editor, the action ranking).

`docs/quilt-keyboard-shortcuts.md:145` lists `t r` → "Toggle right sidebar" as a **not-yet-implemented** shortcut. Logseq parity demands it.

### Existing tests around the right side

- `shared/components/__tests__/AppShell.test.tsx` and `__tests__/AppShell.test.ts` — shell-level integration
- `features/dashboard/__tests__/` — preset/visibility tests
- `features/cognitive/__tests__/` — per-panel tests
- `BlockPropertiesPanel.test.tsx` — unit tests for the panel itself
- `BacklinksPanel` — has tests in the features dir
- E2E: no dedicated spec for the right sidebar as a unified surface; coverage is per-feature

---

## Context Quality

- **Level: C2** — Some durable knowledge (ADR-0030 §14 is fully ratified, ADR-0020 contradicts, migration plan Phase 8 is on file, reversa analysis is solid). The C2→C3 gap is the missing implementation pieces (no global selection state, no right sidebar contract, no test fixture).
- **Evidence Present:**
  - ADR: `docs/adr/0030-graph-space-journal-first-lifecycle.md` §14, `docs/adr/0020-property-editing-surface-fixed-header.md`
  - Plan: `docs/graph-space-migration-plan.md` Phase 8
  - Roadmap: `docs/ROADMAP.md` line 278 (`GS-8 🔲`)
  - Domain terms: panel derecho contextual, selección activa, contexto de página/journal, contexto general del Graph
  - Constraints: visible by default, ≤ 1 main action, persisted preference
  - Test infra: `PanelVisibilityContext` + presets, command registry, BlockContext (per-row)
  - Reversa: `cp__right-sidebar` 320-70% viewport, `t r` shortcut, shift-click to open
- **Missing Context:**
  - Decision on ADR-0020 vs ADR-0030 property-surface conflict (must be raised in proposal)
  - Definition of "active selection" (block-level, page-level, or both? cleared on navigation? cleared on ESC? multi-select?)
  - Definition of "high confidence" for the 0-or-1 main action (heuristic, server-computed, user-trained?)
  - Mobile/touch UX for a contextual sidebar (right-side affordance vs bottom sheet, similar to BacklinksPanel:893-926)
  - E2E spec for the right sidebar as a unified surface
- **Recommended Effort:** **deepen** — the proposal phase must explicitly resolve the ADR-0020 conflict and define the selection-state contract before writing specs.

---

## Knowledge Coverage

| Class | Status | Evidence | Gap Impact |
|------|--------|----------|------------|
| Roadmap/Backlog | present | `docs/ROADMAP.md:278` (GS-8 🔲), `docs/graph-space-migration-plan.md` Phase 8, Phase 10 line 265-286 | Clear target; no ambiguity on what GS-8 is |
| Work Items | stale | No SDD change folder yet for GS-8 (only `sddk/graph-space-metadata-gs7/` exists). No OpenSpec change folder for GS-8 either. | Blocks parallel work; needs a new `sddk/contextual-right-sidebar-gs8/` and a new `openspec/changes/contextual-right-sidebar-gs8/` skeleton during propose |
| Architecture/ADRs | present + conflict | ADR-0030 §14 ratified; ADR-0020 contradicts (property editing surface). Both are accepted. | **Hard blocker for spec** — must be reconciled in proposal |
| Ownership | present (inferred) | Frontend = `quilt-ui/src/features/`; backend = no new endpoints needed (all data already exposed via `/api/v1/blocks/:id/properties`, `/api/v1/blocks/:id/annotations`, etc.) | Low risk; mostly a frontend change |
| Learnings | present | `sddk/graph-space-metadata-gs7/spec.md` (prior kernel flow ran cleanly), `sdd/explore/graph-space-metadata-gs7/explore.md` (explore format precedent) | Pattern to follow; the kernel proposal/spec cadence works |

---

## Problem Taxonomy

| Axis | Applies | Evidence |
|------|---------|----------|
| Domain modeling | **Yes** | New concept: "active selection" (currently absent). Must define scope (block-level, page-level, multi), lifetime, propagation rules. |
| Boundary/seam | **Yes** | Two right-side columns (`BacklinksPanel` + `CognitivePanels`) need to collapse into one contextual surface, while keeping each panel as a section. Reuse `PanelVisibilityContext` and add a new `RightSidebarContext` (or extend the existing one with priority fields). |
| Coupling/connascence | **Yes** | The selection state is read by: BacklinksPanel, BlockPropertiesPanel, cognitive panels, command center, action ranker. A new context will be a hub — must be designed with stable contract (Value Object) to avoid god-object smell. |
| API contract | **No** (v1) | No new server endpoints needed. All data (block properties, page metadata, backlinks, annotations, comments) is already exposed. Possible follow-up: `/api/v1/right-sidebar/actions` (server-ranked main action) but defer. |
| Refactor/legacy | **Yes** | (a) `BlockPropertiesPanel` is currently inline; the right sidebar version is a sibling, not a move. Both may coexist temporarily. (b) `BacklinksPanel` already lives in the right side — it should become one section of the new contextual sidebar, not be replaced. (c) `CognitivePanels` collapses into the same column. |
| Event/CQRS | **Partial** | The 0-or-1 main action is event-shaped (suggested action with confidence). v1 can be client-side heuristic; if the action comes from the agent-room in the future, an event/CQRS pattern is needed. |
| Testing | **Yes** | Need: unit tests for the selection reducer, the priority resolver, the section composer, the property editor (when mounted in the sidebar), and a new E2E spec for the unified surface. The existing `AppShell.test.tsx` and `PanelVisibilityContext` tests are starting points. |
| Security/operations | **No** | No new auth surface, no PII, no external data. |

---

## Domain Language And Invariants

- **Domain Language (resolved):**
  - **Panel derecho contextual** — single right-side column in desktop layout
  - **Selección activa** — what the right sidebar is currently prioritizing (block, page, journal, or graph)
  - **Resolvedor de contexto** — derives the contextual payload from the active selection
  - **Acción principal** — at most one, only when confidence is high
  - **Contexto de página/journal** — fallback when no block is selected
  - **Contexto general del Graph** — bottom fallback (e.g., Agent Activity, Cognitive Graph)
  - **Panel-first** — for properties: the right sidebar is the primary surface (per ADR-0030 §14)
  - **Preferencia global de visibilidad** — sidebar open/closed state persists across sessions
- **Domain Language (ambiguous / needs resolution in proposal):**
  - "Selección activa" — does it include a page-level fallback when no block is selected? My read: yes, by the priority list (§14: 1) selection, 2) page/journal, 3) graph). But the spec must say so explicitly.
  - "Acción principal con alta confianza" — what is "alta confianza"? Heuristic on (a) is there a clear intent match, (b) is the action reversible, (c) is the user idle? This is a design question.
  - "Edición rica de properties" — does "rica" mean the existing BlockPropertiesPanel (typed, NL-date, derived/immutability) is moved into the sidebar, or is it a new richer editor? "Rica" reads as "rich" — same editor, different mount point.
- **Invariants (from ADR-0030 §14, must hold):**
  - Right sidebar is visible by default in desktop
  - Right sidebar is collapsible/hideable
  - Visibility is persisted as a global preference
  - Priority: selection > page/journal > graph
  - Property editing is panel-first (primary surface)
  - At most one main action, only when confidence is high
  - The sidebar also hosts: metadata, contextual actions, suggestions, semantic context
- **Invariants (from ADR-0020, currently holds but conflicts):**
  - Properties are never inline in block content (category error)
  - Block focus populates the same header (no sidebar)
  - Header has exactly 2 states (empty/populated) — CRDT simplicity
- **Unknowns (explicit):**
  - Is the "fixed header" retired, narrowed, or kept alongside the right sidebar?
  - Does the existing `BacklinksPanel` and `CognitivePanels` move into the new contextual sidebar as sections, or do they stay as separate columns and the new sidebar becomes a third column?
  - When the user navigates to `/settings`, `/graph`, or `/pages`, does the right sidebar show Graph context, or hide?

---

## Knowledge Gaps

- **ADR-0020 ↔ ADR-0030 conflict** — both ADRs are accepted and contradict on the property editing surface. Blocks writing a clean spec.
- **Active selection contract** — no definition of "what is selected", lifetime, scope (block-level only? page-level fallback?), multi-select handling. Blocks design.
- **Action ranking heuristic** — no implementation of "main action with high confidence". v1 can be client-side (deterministic, testable); future server integration is open. Blocks acceptance criteria.
- **Mobile treatment** — ADR-0030 §14 says "En desktop". Mobile is out of scope for the panel itself, but the existing pattern (BacklinksPanel as bottom sheet on mobile, `AppShell.tsx:893-926`) is the precedent. Needs explicit "mobile = bottom sheet, desktop = column" decision.
- **Empty state** — what does the right sidebar show when no selection, no page, no graph? Today both panels are blank. Proposal must define the empty state.
- **Stale explore folder for GS-8** — only `sddk/graph-space-metadata-gs7/` exists. Propose phase should create `sddk/contextual-right-sidebar-gs8/` (proposal + spec + design + tasks) and a matching `openspec/changes/contextual-right-sidebar-gs8/`.
- **E2E coverage** — no spec for the unified right sidebar. Test pyramid gap.

---

## Affected Areas

- `quilt-ui/src/shared/components/AppShell.tsx` (compose right side; default visibility; mobile bottom sheet) — **major change**
- `quilt-ui/src/features/dashboard/PanelVisibilityContext.tsx` (extend with priority fields, or add new `RightSidebarContext`) — **major change**
- `quilt-ui/src/features/dashboard/presets.ts` (default preset = `[sidebar, right-sidebar]` instead of `[sidebar, backlinks]`) — **major change**
- `quilt-ui/src/features/sidebar/Sidebar.tsx` (left sidebar; unrelated but coexists) — **minor**: ensure toggles compose
- `quilt-ui/src/features/backlinks/BacklinksPanel.tsx` (move into new shell as a section) — **major change** (refactor mount, not delete)
- `quilt-ui/src/features/cognitive/CognitivePanels.tsx` (move into new shell as sections) — **major change** (refactor mount, not delete)
- `quilt-ui/src/features/properties/BlockPropertiesPanel.tsx` (mount inside new shell when a block is selected) — **minor**: extract as a section
- `quilt-ui/src/features/outliner-tiptap/BlockRow.tsx:236` (per-row `showProperties` flag — keep, the inline panel can remain as a quick-edit alternative or be deprecated) — **decision needed**
- `quilt-ui/src/features/outliner-tiptap/PageView.tsx` (push selection state to the new context) — **major change**
- `quilt-ui/src/features/command-center/builtin-commands.ts` (add `layout/toggle-right-sidebar` and `layout/switch-to-default` updated default preset) — **minor**
- `quilt-ui/src/features/cognitive/WeeklyReview.tsx`, `Serendipity.tsx` (consume the new action ranker for "main action") — **minor**
- New: `quilt-ui/src/features/right-sidebar/` (composer, sections, action ranker, selection context) — **new module**
- `quilt-ui/src/features/annotations/AnnotationPanel.tsx` (could mount as a section) — **follow-up**
- `quilt-ui/src/features/comments/` (could mount as a section) — **follow-up**
- `docs/quilt-keyboard-shortcuts.md:145` (implement `t r` toggle) — **minor**
- `docs/adr/0020-property-editing-surface-fixed-header.md` (supersede, narrow, or keep — design decision) — **ADR change**

---

## Options

| Option | Pros | Cons | Effort |
|--------|------|------|--------|
| **A. Single unified right sidebar; new selection context; supersede ADR-0020** | Cleanest alignment with ADR-0030 §14. One column, one resolver, one persistence. The Logseq pattern. | Requires retiring the fixed header (large UX surface change), big PR. | High (3-4 weeks) |
| **B. Single unified right sidebar; keep fixed header for page-level properties; new context for block-level selection** | Resolves the ADR-0020 ↔ ADR-0030 conflict by scoping each. Lower risk. | Two surfaces still exist; users may be confused which to use. Medium code overhead. | Medium-High (2-3 weeks) |
| **C. New right sidebar for selection+actions+metadata; keep backlinks + cognitive as separate columns to the right of it** | Minimum risk: existing surfaces unchanged, new one is purely additive. | Width pressure on small screens. Doesn't match ADR-0030's "single right column" reading. | Medium (1.5-2 weeks) |
| **D. New right sidebar for selection+actions+metadata; rename the existing pair (backlinks + cognitive) to "insights" and merge them into the new shell as sections** | Matches Logseq pattern, satisfies all ADR-0030 §14 requirements, minimal new endpoints. | Largest code refactor of the three (A's downside); but a clean migration with feature flags. | High (3 weeks) |

The persona would recommend **B** for v1 (scoped, low risk, honors both ADRs) with a follow-up that narrows ADR-0020 or supersedes it in a later phase.

---

## Entropy Envelope

- **Method:** heuristic (no CogniCode MCP available in this explore phase)
- **Coupling risk:** **medium-high**
  - The new selection context will be read by 6+ components (BacklinksPanel, BlockPropertiesPanel, CognitivePanels, command center, action ranker, command center)
  - The current `BlockContext` (per-row) and the new global `SelectionContext` will both exist; clear naming and ownership required to avoid confusion
  - `PanelVisibilityContext` already serves as a precedent for global UI state; the new context should follow the same pattern (localStorage persistence, custom DOM event, hydration effect)
- **Notes:**
  - Connascence of execution: when the user changes selection, the sidebar must re-resolve. Every section that reads selection is implicitly coupled to the same event. Mitigation: a single `useContextSelection()` hook with a stable contract.
  - OCP risk: the new sidebar's section list is dynamic (BacklinksPanel, BlockPropertiesPanel, CognitivePanels, AnnotationsPanel, CommentsThread — all could be sections). Adding a new section should not require editing the shell. Mitigation: a registry pattern (`RightSidebarSection[]` registered by each feature).
  - The "main action with high confidence" slot is a hot interface — every cognitive feature will want to plug in. Mitigation: a single `useMainAction()` contract, with a confidence score; only the top one is rendered.

---

## Recommendation

**Recommended path:** Option **B** (single unified right sidebar with `BacklinksPanel` + `CognitivePanels` migrated in as sections, plus a new selection context that drives a `BlockPropertiesPanel` section when a block is selected). Keep the fixed header from ADR-0020 for *page-level* properties only; let the right sidebar own *block-level* properties (the primary surface per ADR-0030 §14). This:

- Resolves the ADR-0020 ↔ ADR-0030 conflict by scoping each ADR to a different property surface (page vs block)
- Aligns with Logseq's `cp__right-sidebar` pattern
- Is achievable in 2-3 weeks with the existing `PanelVisibilityContext` machinery
- Leaves room to retract the fixed header in a follow-up if block-level property editing in the sidebar proves sufficient

**Specific proposal-phase deliverables:**

1. Define the `SelectionContext` contract (scope, lifetime, propagation, multi-select)
2. Define the `RightSidebarSection` registry pattern
3. Define the "main action" heuristic (v1: deterministic, no server roundtrip)
4. Write the ADR-0020 ↔ ADR-0030 reconciliation note (a new ADR-0031 or an amendment to ADR-0020)
5. Sketch the migration of `BacklinksPanel` + `CognitivePanels` into sections of the new shell
6. Add a default preset update: `default: [sidebar, right-sidebar]`
7. Define the empty state (no selection, no page: show Graph context)
8. Define the mobile treatment (bottom sheet, mirroring `AppShell.tsx:893-926`)

---

## Ready For Proposal

**Yes** — with the caveat that the proposal phase MUST address the ADR-0020 ↔ ADR-0030 conflict and the "active selection" contract before writing specs. Context quality is C2 (sufficient durable knowledge, two critical open questions). The kernel proposal cadence (`sddk/graph-space-metadata-gs7/proposal`) is a clear precedent to follow.

Recommended next phase: `sddk-propose` with the new change folder `sddk/contextual-right-sidebar-gs8/` and matching `openspec/changes/contextual-right-sidebar-gs8/`.

---

## Return Envelope

**Status:** success
**Summary:** Explored GS-8 — current right side is two independent columns (BacklinksPanel + CognitivePanels) with no selection state model. Block properties are edited inline (BlockPropertiesPanel rendered below the block), not in the sidebar. ADR-0030 §14 and ADR-0020 directly conflict on the property editing surface; this must be resolved in proposal.
**Artifacts:**
- Engram `sddk/contextual-right-sidebar-gs8/explore` (this file)
- Path: `sddk/contextual-right-sidebar-gs8/explore.md`
**Next:** sddk-propose (with ADR-0020 ↔ ADR-0030 reconciliation as a precondition)
**Risks:**
- ADR-0020 ↔ ADR-0030 conflict is unaddressed in either ADR; the proposal must reconcile.
- The "active selection" concept is brand new — no precedent in the codebase. Wrong contract = expensive refactor.
- The new selection context will be a hub; connascence risk is medium-high. Mitigate with a stable hook contract and a section registry.
- Mobile treatment is undefined; copying the BacklinksPanel bottom-sheet pattern is the safest default.
- Option C (additive, no merge) is the lowest-risk path; the persona recommends B (scoped) for value but C is acceptable if B is too large.
**Skill Resolution:** paths-injected (sddk-explore skill loaded by orchestrator)
**Context Quality:** C2
