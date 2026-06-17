# Kernel Design: GS-8 — Contextual Right Sidebar

> Source: `sddk/contextual-right-sidebar-gs8/spec.md` (8 capabilities), `sddk/contextual-right-sidebar-gs8/explore.md`, proposal Engram #2277
> Date: 2026-06-17
> Entropy method: heuristic (CogniCode MCP unavailable)

## Context Reuse Check
| Input | Status | Notes |
|-------|--------|-------|
| Knowledge coverage | present | ADR-0030 §14, ADR-0020 (text verified), migration plan Phase 8, reversa `cp__right-sidebar`, ROADMAP:278 all reused. No new exploration needed. |
| Exploration | present | Explore #2276 reused verbatim — current two-column shell, BlockContext per-row, PanelVisibilityContext machinery all confirmed in code. |
| Proposal/spec alignment | ok | Proposal (Engram #2277) resolved ADR-0020↔0030 (Option B: narrow, not rescind), SelectionContext contract, registry, deterministic ranker. Spec codifies all 8 capabilities. |
| Code verification | ok | AppShell.tsx:928/944 (two asides), PanelVisibilityContext.tsx (dispatchDashboardChange, DASHBOARD_EVENT, presets), presets.ts:55 (`default: [sidebar, backlinks]`), CognitivePanels.tsx:39-64 (8 flags, returns null), AppShell.tsx:162-201 (leader key hardcoded to `'g'`, `executeGoTo(combo)`), BlockContext.tsx (per-row, no global state), builtin-commands.ts (`layout/toggle-*` via dispatchDashboardChange). |
| Context quality | C2 | Durable ADRs + verified code. C2→C3 gap = implementation; this design closes it. Effort = verify + targeted deepening on seams. |
| Problem taxonomy | present | Axes reused: domain-modeling (SelectionContext), boundary/seam (2 cols→1 shell), coupling (6+ consumers), refactor (BacklinksPanel/Cognitive migrate as sections), testing (new E2E). |
| Domain language | present | All resolved by proposal: selección activa includes page-fallback; acción principal = deterministic v1; edición rica = existing BlockPropertiesPanel remounted. |
| Recommended effort | verify | Proposal already resolved open Qs; design codifies interfaces, seams, migration slices. |

## Technical Approach

Turn the current **two independent `<aside>` columns** (`BacklinksPanel` + `CognitivePanels`, 640px combined) into **one unified, selection-aware right sidebar** driven by a new ephemeral `SelectionContext` and a singleton `RightSidebarSectionRegistry`. Strategy = Option B from explore: narrow (not rescind) ADR-0020 — the fixed header keeps **page-level** properties; the sidebar owns **block-level** properties (ADR-0030 §14 primary surface). All existing panels survive as registered sections; no endpoints change.

The three new seams are: (1) `SelectionContext` (ephemeral React context + reducer + pure resolver), (2) `RightSidebarSectionRegistry` (module singleton, registration-time side effects via a barrel), (3) `rankMainAction` (pure function). The shell composes them; migration is feature-flagged so the two-column layout and the new shell never co-render.

## Knowledge Impact
- Durable artifacts reused: ADR-0030 §14, ADR-0020, `docs/graph-space-migration-plan.md` Phase 8, `docs/quilt-keyboard-shortcuts.md:145`, `PanelVisibilityContext.tsx` (machinery + `DASHBOARD_EVENT`), `presets.ts`, reversa `cp__right-sidebar`.
- Artifacts needing supersession: ADR-0020 — its "block-focus-populates-header" interaction is superseded; the ADR is **narrowed** to page-level only via new ADR-0031 (draft below). Not rescinded.
- Memory-only learnings consulted: explore #2276 (entropy envelope, Options A–D), proposal #2277 (heuristic weights deferred to here).

## Applied Lenses
| Lens | Delegation | Status | Why Applied | Design Impact |
|------|------------|--------|-------------|---------------|
| base-discipline | kernel | applied | Always active | Context reuse verified; depth capped by C2; entropy reported with method. |
| entropy-sdd | skills/entropy-sdd Protocol C | deepened | Mandatory; high-coupling hub + singleton registry | `SelectionContext` typed as discriminated union (low leakage); registry gets a `createSectionRegistry()` factory + default singleton so tests isolate; ranker pure (optimal bottleneck). |
| coupling/connascence | kernel heuristic | deepened | 6+ consumers of selection; module singleton = identity connascence risk | Stable `useSelection()` hook contract; `resetRegistryForTests()` exported; registry is a leaf (no feature imports) → dependency direction points inward. |

## Invariants And Constraints
| Invariant / Constraint | Enforcement Point | Verification |
|------------------------|-------------------|--------------|
| Selection ephemeral (no localStorage) | `SelectionContext` reducer never calls storage | Unit: assert no `localStorage.setItem` on selection path |
| Priority block > page/journal > graph | `resolveSelection()` pure, route-key guard | Unit: 4 fallback scenarios |
| Clear block on route change | reducer `ROUTE_CHANGE` action keyed by `useLocation` | Unit: route change → BlockSelection gone |
| Clear block on ESC | global keydown listener dispatches `CLEAR_BLOCK` | Unit: ESC → PageSelection fallback |
| ≤ 1 main action, confidence ≥ 0.7 | `rankMainAction` pure fn + threshold | Unit: 6 ranker scenarios |
| Header unchanged on block select | header reads page props only; ADR-0031 | Unit + E2E: header asserts page props |
| Registry ordered (priority asc, then reg order) | `getSectionsForSelection` stable sort | Unit: equal-priority ordering |

## Architecture Decisions
| Decision | Choice | Alternatives Considered | Rationale |
|----------|--------|-------------------------|-----------|
| Selection store shape | `useReducer` + pure `resolveSelection(state, route)` | Single context value updated imperatively | Reducer gives deterministic transitions; route-key guard makes "clear on nav" implicit + testable. |
| Registry lifecycle | Module singleton + barrel side-effect imports + `createSectionRegistry()` factory | React context provider | Spec mandates module-load registration (no provider); factory enables test isolation. |
| Leader key `t` | Generalize `useGlobalShortcuts` to `leaders: Record<string, handler>` | Duplicate state machine | Avoids the `t*` family conflict flagged in spec open Qs; one timeout, two leaders. |
| ADR-0020 fate | Narrow (page-level only) via ADR-0031 | Rescind / keep both fully | Rescind loses page-property precedent; both-fully contradicts §14. Narrow honors both. |

### ADR-0031 (draft — to be written to `docs/adr/0031-right-sidebar-property-split.md` during apply)
- **Status:** proposed (supersedes part of ADR-0020). **Title:** Right Sidebar Property Split — Block-Level in Sidebar, Page-Level in Header.
- **Context:** ADR-0020 (accepted) made the fixed header the sole property surface and rejected the right sidebar ("two-surface editing model"). ADR-0030 §14 (accepted later) mandates the right sidebar as the **primary** surface for block-level property editing. Direct conflict.
- **Decision:** (1) ADR-0020 is **narrowed** — it retains authority for **page-level** properties in the fixed header. (2) The ADR-0020 behavior "block-focus populates the same header" is **superseded** — block-level properties render exclusively in the right sidebar on `BlockSelection`. (3) Both surfaces coexist; each is authoritative for its scope.
- **Why:** Resolves the contradiction without losing ADR-0020's CRDT/category-boundary rationale (still valid for page metadata); aligns with ADR-0030 §14 and the Logseq precedent; both invariants hold.
- **Consequences:** + Clear scope per surface; sidebar is selection-reactive. − Two editing surfaces exist (mitigated: distinct scopes, never the same property). Follow-up: deprecate inline `BlockPropertiesPanel` in BlockRow once sidebar proves sufficient.

## Data Flow
```
BlockRow.onClick ─┐
useLocation ──────┼─▶ SelectionProvider(reducer) ─▶ resolveSelection ─▶ Selection (VO)
ESC keydown ──────┘            │                                          │
                               ▼                                          ▼
              RightSidebarSectionRegistry.getSectionsForSelection(Selection)
                               │                          │
                               ▼                          ▼
                  visible Sections[] ──▶ rankMainAction(Selection, actions) ─▶ 0|1 main action
                               │
                               ▼
                   <RightSidebarShell> (reads PanelVisibility 'right-sidebar')
```

## File Changes
| File | Action | Description |
|------|--------|-------------|
| `features/right-sidebar/selection/types.ts` | new | `Selection` discriminated union (Block/Page/Journal/Graph/None) |
| `features/right-sidebar/selection/SelectionContext.tsx` | new | Provider + `useReducer` + `useSelection()` + `useSelectionActions()` |
| `features/right-sidebar/selection/resolveSelection.ts` | new | pure `resolveSelection(state, route)` |
| `features/right-sidebar/sections/registry.ts` | new | `createSectionRegistry()` factory + default singleton + `resetRegistryForTests()` |
| `features/right-sidebar/sections/types.ts` | new | `RightSidebarSection`, `SectionAction` (`kind: action\|suggestion`) |
| `features/right-sidebar/sections/index.ts` | new | barrel: side-effect imports for each registering feature |
| `features/right-sidebar/rankMainAction.ts` | new | pure ranker, threshold 0.7, suggestion −0.3 |
| `features/right-sidebar/RightSidebarShell.tsx` | new | unified `<aside>` (320px) + main-action slot + tabs + empty state + mobile bottom sheet |
| `features/right-sidebar/RightSidebarEmptyState.tsx` | new | static empty state |
| `features/backlinks/registerSidebarSection.ts` | new | registers `backlinks` (priority 200), predicate page\|journal |
| `features/cognitive/registerSidebarSections.ts` | new | registers 8 cognitive sections (priority 300–370), dual-gate (flag + predicate) |
| `features/properties/SidebarPropertiesSection.tsx` | new | mounts `BlockPropertiesPanel` on `BlockSelection` |
| `shared/components/AppShell.tsx` | modify | replace 2 `<aside>` with `<RightSidebarShell>`; generalize `useGlobalShortcuts` to `g`+`t` leaders; add ESC listener; wrap tree in `<SelectionProvider>` |
| `features/dashboard/presets.ts` | modify | `PanelId` += `'right-sidebar'`; `default: [sidebar, right-sidebar]` |
| `features/dashboard/PanelVisibilityContext.tsx` | modify | add `'right-sidebar'` to `PANEL_LABELS` + `DEFAULT_PANELS` |
| `features/cognitive/CognitivePanels.tsx` | retire | wrapper removed; content lives in sections |
| `features/command-center/builtin-commands.ts` | modify | add `layout/toggle-right-sidebar` (dispatchDashboardChange toggle) |
| `docs/adr/0031-right-sidebar-property-split.md` | new | ADR-0031 (draft above) |
| `tests/e2e/spec/right-sidebar.spec.ts` | extend | unified-surface scenarios |

## Interfaces / Contracts
```ts
type Selection =
  | { type: 'block'; blockId: string; pageName: string }
  | { type: 'page'; pageName: string }
  | { type: 'journal'; date: string }
  | { type: 'graph' }
  | { type: 'none' };

interface RightSidebarSection {
  sectionId: string;
  priority: number;                 // lower renders first
  predicate: (s: Selection) => boolean;
  render: (s: Selection) => ReactNode;
  actions?: SectionAction[];        // optional, feeds ranker
}
interface SectionAction { id: string; label: string; kind: 'action' | 'suggestion'; onExecute: () => void; }

// registry.ts (leaf — no feature imports)
function createSectionRegistry(): SectionRegistry;   // for tests
const rightSidebarSections: SectionRegistry;          // default singleton
interface SectionRegistry {
  register(s: RightSidebarSection): void;
  getSectionsForSelection(s: Selection): RightSidebarSection[]; // stable sort
  reset(): void;                                                  // test-only
}

// rankMainAction.ts — pure
function rankMainAction(s: Selection, sections: RightSidebarSection[]): { action: SectionAction; confidence: number } | null;
```

## Entropy Constraints
| Interface/Module | Risk | Constraint |
|------------------|------|------------|
| `SelectionContext` | I(Name)≈log2(6)≈2.6 bits (6+ consumers) | Stable `useSelection()` hook; `Selection` is an opaque VO (no reducer internals leak). |
| `rightSidebarSections` singleton | Identity connascence; test pollution | `createSectionRegistry()` factory + `reset()`; tests build isolated instances. |
| `registry.ts` ↔ sections | Import cycle if registry imports features | Registry is a leaf; only the barrel imports features. |
| `useGlobalShortcuts` | Execution connascence (leader ordering) | Single timeout, `leaders` map keyed by char; `t` only when not in contentEditable. |

## Testing Strategy
| Layer | What To Test | Approach |
|-------|--------------|----------|
| Unit | `resolveSelection` (4 fallbacks + route-key guard + ESC) | Vitest, inject fake route |
| Unit | `rankMainAction` (6 spec scenarios: null, single match, mismatch, multi-priority, suggestion threshold, sub-threshold) | Pure fn, table-driven |
| Unit | registry ordering (priority asc, reg-order tie, predicate filter) | `createSectionRegistry()` isolated |
| Unit | reducer transitions (SELECT_BLOCK, CLEAR_BLOCK, ROUTE_CHANGE) | `renderHook` + act |
| Component | `<RightSidebarShell>` renders sections, main-action slot, empty state | Testing Library, mock SelectionProvider |
| Integration | Backlinks/cognitive sections resolve pageName from each Selection type | InMemoryProvider |
| E2E | `t r` toggle, block-select shows properties, header unchanged, mobile bottom sheet | Playwright (`tests/e2e/spec/right-sidebar.spec.ts`), Bearer auth |

## Migration / Rollout
Feature-flagged, never co-render the two layouts:
1. **Add (no UI):** new `features/right-sidebar/` module — types, reducer, resolver, registry factory, ranker. Unit-tested, unmounted.
2. **Plumb:** add `'right-sidebar'` to `PanelId`/`PRESETS.default`/`PANEL_LABELS`; mount `<SelectionProvider>` + import the barrel (side effects register sections); add `layout/toggle-right-sidebar`.
3. **Build shell hidden:** `<RightSidebarShell>` complete but gated behind a flag; BlockRow dispatches `SELECT_BLOCK`; generalize leader key to `g`+`t`.
4. **Swap (flag on):** AppShell renders `<RightSidebarShell>`; remove the two `<aside>` columns; retire `CognitivePanels.tsx`; inline `BacklinksPanel` column deleted.
5. **Stabilize:** empty state, `t r`, ESC, E2E green; flag default-on; remove flag.
Rollback = flag off → original two-column path (kept until step 5).

## Open Questions
- None blocking. Deferred (spec open Qs, non-blocking): accordion vs stack UX for section tabs; mobile cognitive section selection; inline `BlockPropertiesPanel` deprecation timeline; `t*` family (`t l`/`t d`/`t b`) extension.
