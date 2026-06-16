// ─── presets — DashboardLayout panel presets ─────────────────────
//
// The DashboardLayout is a per-workspace "which panels are visible
// right now" preset. We do NOT model a "Work Mode" entity (see the
// ADR-DRAFT `dashboard-layout-no-work-modes.md`): a preset is just
// a `Set<PanelId>` plus a name, and the user can switch between
// them or edit the current set freely.
//
// This module is pure data — no React, no localStorage. Persistence
// and React glue live in `PanelVisibilityContext.tsx`.

/** Every panel the DashboardLayout knows about. */
export type PanelId =
  | 'sidebar'
  | 'backlinks'
  | 'agent-activity'
  | 'agent-room'
  | 'outline'
  | 'structural-graph'
  | 'semantic-insight'
  | 'decay-monitor'
  | 'weekly-review'
  | 'serendipity'
  | 'cognitive-graph'

/**
 * Re-export of the canonical `PANEL_LABELS` from
 * `PanelVisibilityContext.tsx`. The constant is *defined* in the
 * context module (single source of truth) and re-exported here so
 * existing imports from `./presets` keep resolving. Do not redeclare
 * a second `PANEL_LABELS` value — that would create the dual-source
 * drift this re-export is designed to prevent.
 */
export { PANEL_LABELS } from './PanelVisibilityContext'

/** A preset id is the key into the `PRESETS` table. */
export type PresetId = 'default' | 'focus' | 'review'

/**
 * The three presets shipped with the app.
 *
 *   - `default` — the day-to-day surface: sidebar + backlinks.
 *   - `focus`   — full-width writing, no sidebar.
 *   - `review`  — sidebar + backlinks + agent activity +
 *                 structural graph. Used when reviewing what
 *                 agents added during a session. The semantic-
 *                 insight panel stays off by default — it requires
 *                 agent-authored insight blocks to be useful.
 *
 * Keep this object frozen in *intent* (don't mutate the inner sets
 * at runtime); `getPreset` returns a fresh Set every call so the
 * caller is free to mutate it.
 */
export const PRESETS: Record<PresetId, ReadonlySet<PanelId>> = {
  default: new Set<PanelId>(['sidebar', 'backlinks']),
  focus: new Set<PanelId>(['backlinks']),
  review: new Set<PanelId>([
    'sidebar',
    'backlinks',
    'agent-activity',
    'structural-graph',
  ]),
}

/** Every preset id, in display order. */
export const PRESET_ORDER: readonly PresetId[] = ['default', 'focus', 'review']

/** Human-readable label for a preset id. */
export const PRESET_LABELS: Record<PresetId, string> = {
  default: 'Default',
  focus: 'Focus',
  review: 'Review',
}

/**
 * Resolve a preset by id. Returns a *fresh* Set every call so
 * callers can mutate the result without poisoning the table.
 *
 * Unknown ids (a stale value in localStorage, a typo in a future
 * keyboard shortcut) fall back to the `default` preset rather than
 * throwing — the provider should never crash on a bad preset name.
 */
export function getPreset(id: PresetId): Set<PanelId> {
  const preset = PRESETS[id] ?? PRESETS.default
  return new Set(preset)
}

/**
 * Best-effort: which preset is `set` closest to? Used by the
 * LayoutMenu to highlight the active preset button when the user
 * has been editing individual checkboxes. Returns `null` when the
 * set is a custom layout that does not match any preset.
 */
export function findClosestPreset(set: ReadonlySet<PanelId>): PresetId | null {
  for (const id of PRESET_ORDER) {
    const preset = PRESETS[id]
    if (preset.size !== set.size) continue
    let same = true
    for (const panel of preset) {
      if (!set.has(panel)) {
        same = false
        break
      }
    }
    if (same) return id
  }
  return null
}
