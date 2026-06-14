# ADR-0023: Block Renderer Registry — extensible block rendering architecture

Status: implemented

## Context

Quilt's `BlockRow` was 1855 lines with 14 hardcoded visual variations: task marker badges, agent-run headers, saved-view dispatch, priority pills, created-by badges, annotation badges, property strips, inline property badges — all rendered inline via React conditionals inside the component. Adding a new visual behavior meant editing `BlockRow`'s render tree.

Quilt inherits Logseq's outliner model where `marker` (task state) and `blockType` (structural format) are ORTHOGONAL fields. A heading can be `TODO` — it should show a checkbox AND heading formatting. The rendering system must compose multiple concerns.

Research (2026-06-11) confirmed two industry patterns:

- **Logseq**: marker-overlay pattern — `:block/marker` is a separate attribute applied on any block type.
- **AFFiNE/BlockSuite**: BlockSpec registry pattern — `BlockSpec = { schema, service, view }` registered by type.
- **Notion**: blockType-as-task — `to_do` is a dedicated block type with `checked` property.
- **Recommendation for Quilt**: marker-overlay (Logseq) + registered renderers (AFFiNE).

Five independent "what kind of block is this?" axes exist and were disconnected:

1. `blockType` (11 variants) — structural format
2. `marker` (6 values) — task state
3. `properties.type` → WASM strategy (5 names) — role
4. `properties.template` → CardRenderer (4 shapes) — card wrapper
5. Inline property rendering (`PropertyStrip`, `InlinePropertyBadges`)

## Decision

**BlockRendererRegistry** — an extensible registry where renderers control specific rendering "slots" via composable contributions. `BlockRow` queries the registry; new visual behaviors register without touching `BlockRow`.

### Four composable slots

1. `renderBullet` (exclusive, highest priority wins) — replace the bullet dot with a custom element.
2. `renderBeforeContent` (additive, all matching contribute) — render between bullet and content.
3. `wrapContent` (composable, outermost = lowest priority) — wrap the content area.
4. `getBlockStyle` (merged CSS) — additional block row styles.

### Key design principles

- **Marker is data the renderer reads, not a dispatch key.** A `HeadingRenderer` wraps content in h1/h2/h3; a `TaskRenderer` adds a checkbox when `marker` is set. Both can match the same block.
- **Priority system**: higher priority wins for exclusive slots, innermost for wrappers.
- **TaskRenderer** (priority 10) is the first built-in: replaces the bullet with an accessible checkbox (`role="checkbox"`, `aria-checked`), applies dim/strikethrough for `Done`/`Cancelled`.
- **5-step marker cycle**: `null → Todo → Doing → Done → Cancelled → null` (replaces the previous 3-step `null → Todo → Done`).

### Module-level registration

```typescript
const blockRegistry = new BlockRendererRegistry();
blockRegistry.register(TaskRenderer);
// Future: blockRegistry.register(HeadingRenderer), CodeRenderer, etc.
```

## Considered Options

1. **Registry per-blockType only** — rejected: marker is orthogonal to blockType. A `TODO` heading needs BOTH `TaskRenderer` AND `HeadingRenderer`.
2. **Extend WASM strategy selector to also match on marker** — rejected: strategy is for roles (query/view/agent-run), not visual concerns. Mixing them would couple the WASM module to UI rendering decisions.
3. **Separate `TaskBlock` component outside `BlockRow`** — rejected: Logseq-style outliners need marker overlay on ANY block. A separate component duplicates all of `BlockRow`'s complexity.
4. **Keep hardcoded conditionals but extract to functions** — rejected (what we had before): extensibility requires a registry pattern. Every new renderer touching `BlockRow` was unsustainable.

## Consequences

- `BlockRow` goes from 1855 lines toward a pure compositor — current state removes ~40 lines.
- `MARKER_STYLES` becomes dead code (removed) — `Checkbox` uses its own `MARKER_COLORS`.
- 276/276 existing tests continue to pass.
- 13 inline hardcoded variations remain to migrate to the registry (see ADR-0024).
- `CardRenderer` wraps `BlockRow` from OUTSIDE (via `PageView`) — separate layer, independent of the registry.

## Implementation

- `rendering/types.ts` (58 lines) — `BlockRenderer`, `BlockRendererContext` interfaces.
- `rendering/registry.tsx` (86 lines) — `BlockRendererRegistry` class with priority sorting and slot composition.
- `rendering/Checkbox.tsx` (77 lines) — accessible `BlockCheckbox` with marker-driven colors and states.
- `rendering/TaskRenderer.tsx` (66 lines) — first renderer.
- `BlockRow.tsx` modified — uses registry for bullet + content wrapping; marker badge removed; `isDimmed` removed.

## References

- ADR-0014 — Strategy Selector (WASM strategy is for roles, not visual concerns).
- ADR-0015 — Agent Run block role (rendered via `AgentRunRenderer` candidate).
- ADR-0016 — SavedView block role (rendered via `StrategyViewRenderer` candidate).
- ADR-0022 — Template-driven block cards (separate layer, wraps `BlockRow` from outside).
- ADR-0024 — Property-driven view renderers (next phase of registry adoption).
