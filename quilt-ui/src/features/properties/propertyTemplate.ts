// ──── Property Templates — quilt-roadmap-#13 ────────────────────────────
//
// A `PropertyTemplate` decides which block properties are surfaced
// inline (badges next to the block content) vs. hidden behind the
// side-panel form. Different block types have different defaults —
// tasks surface `status` / `priority` / `due` inline (so you can scan
// a task list at a glance) but a query block keeps its `dsl`
// expression panel-only (the DSL is long, the badge would be ugly).
//
// V1 design rules (see ROADMAP.md §13):
//
//   1. **Default**: every property is panel-only; nothing inline.
//   2. **Tasks** (`blockType === 'todo'`, or `type:: todo` property):
//      `status`, `priority`, `due` are inline. `dsl` is panel-only.
//   3. **Query blocks** (`type:: query` property): `dsl` is
//      panel-only, never inline.
//   4. **Custom**: future template overrides can extend the registry
//      without changing this file (see TEMPLATE_REGISTRY below).
//
// The helpers in this file are pure — no React, no API calls. They
// run during render and during tests, so keep them cheap.

import type { Block } from '@shared/types/api'

/**
 * A template that classifies a block's properties into two
 * presentation buckets.
 *
 *  - `inline`: keys that should render as a clickable badge next to
 *    the block content. Clicking the badge opens an inline editor.
 *  - `panelOnly`: keys that are *never* inline, even if the user adds
 *    them to a block. Used for "structural" properties (e.g. the
 *    query DSL) that are too verbose or too sensitive for a badge.
 *
 * Keys that are in NEITHER list are "default" — they show in the
 * panel and may also be surfaced inline by the default block-type
 * behaviour. Currently the default template puts everything in the
 * panel and nothing inline; the `todo` template moves a small set of
 * keys to inline.
 */
export interface PropertyTemplate {
  /** Keys that should appear as inline badges on the block row. */
  inline: readonly string[]
  /** Keys that are always panel-only (e.g. `dsl`, `template`). */
  panelOnly: readonly string[]
}

/**
 * The default template: nothing inline, nothing panel-only — every
 * key is just a regular panel property.
 */
export const DEFAULT_PROPERTY_TEMPLATE: PropertyTemplate = {
  inline: [],
  panelOnly: [],
}

/**
 * Template for a TODO / task block. `status`, `priority` and `due`
 * become inline badges. `dsl` (when present) is panel-only.
 */
export const TODO_PROPERTY_TEMPLATE: PropertyTemplate = {
  inline: ['status', 'priority', 'due'],
  panelOnly: ['dsl'],
}

/**
 * Template for a query block. The DSL expression is long, so it is
 * panel-only. The block name and other metadata stay default.
 */
export const QUERY_PROPERTY_TEMPLATE: PropertyTemplate = {
  inline: [],
  panelOnly: ['dsl'],
}

// ──── Block-type detection ──────────────────────────────────────────

/**
 * Heuristic: is this block a task?  True if the explicit
 * `blockType` is `todo`, or if the block carries a `type:: todo` /
 * `type:: task` property (a Quilt convention for blocks typed via
 * properties instead of the typed field — e.g. the `/task` slash
 * command writes `type:: task` while the `/todo` slash command and
 * checkbox blocks still write `type:: todo` / `blockType: 'todo'`).
 */
function isTaskBlock(block: Block): boolean {
  if (block.blockType === 'todo') return true
  const t = block.properties?.find(p => p.key === 'type')?.value
  if (t == null) return false
  const norm = String(t).toLowerCase()
  return norm === 'todo' || norm === 'task'
}

/**
 * Heuristic: is this block a query block?  True if it carries
 * `type:: query` in its properties.
 */
function isQueryBlock(block: Block): boolean {
  const t = block.properties?.find(p => p.key === 'type')?.value
  return t != null && String(t).toLowerCase() === 'query'
}

// ──── Public API ────────────────────────────────────────────────────

/**
 * Resolve the active `PropertyTemplate` for a block.  The order of
 * precedence is:
 *
 *   1. Query blocks → `QUERY_PROPERTY_TEMPLATE`
 *   2. Task blocks  → `TODO_PROPERTY_TEMPLATE`
 *   3. Everything else → `DEFAULT_PROPERTY_TEMPLATE`
 *
 * Pure function: same input always yields the same output.
 */
export function getPropertyTemplate(block: Block): PropertyTemplate {
  if (isQueryBlock(block)) return QUERY_PROPERTY_TEMPLATE
  if (isTaskBlock(block)) return TODO_PROPERTY_TEMPLATE
  return DEFAULT_PROPERTY_TEMPLATE
}

/**
 * The keys of the given block's properties that should render as
 * inline badges, in the order the template declares (so
 * `status → priority → due` stays predictable on a task block).
 *
 * If the block has no properties, the result is empty.
 */
export function getInlinePropertyKeys(block: Block): string[] {
  const tpl = getPropertyTemplate(block)
  if (tpl.inline.length === 0) return []
  const present = new Set((block.properties ?? []).map(p => p.key))
  return tpl.inline.filter(k => present.has(k))
}

/**
 * Convenience: the keys of the given block that are *only* allowed
 * in the panel (i.e. the template lists them in `panelOnly`).
 */
export function getPanelOnlyPropertyKeys(block: Block): string[] {
  const tpl = getPropertyTemplate(block)
  if (tpl.panelOnly.length === 0) return []
  const present = new Set((block.properties ?? []).map(p => p.key))
  return tpl.panelOnly.filter(k => present.has(k))
}

/**
 * Type guard-ish: does the template for `block` say that `key`
 * belongs to `bucket`?
 *
 * @param bucket  'inline' | 'panelOnly'
 */
export function isPropertyKeyInTemplate(
  block: Block,
  key: string,
  bucket: 'inline' | 'panelOnly',
): boolean {
  const tpl = getPropertyTemplate(block)
  return (tpl[bucket] as readonly string[]).includes(key)
}
