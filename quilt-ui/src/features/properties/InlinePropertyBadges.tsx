// ──── Inline Property Badges — quilt-roadmap-#13 ────────────────────
//
// A row of small clickable pills that surface a block's *inline*
// properties (per the active `PropertyTemplate`) directly on the
// block row. Click a pill → inline editor appears in place of the
// pill. Enter / blur saves via `api.setBlockProperty`; Escape cancels.
//
// Why a separate component (not inline in BlockRow)?
//   - Smaller test surface — no need to mount the full 1700-line
//     BlockRow to test the badge behaviour.
//   - Reusable from any container that wants to show inline
//     properties (page header, multi-select toolbar, etc.).
//   - Keeps the keyboard / autocomplete / slash-command complexity
//     of BlockRow from leaking into the badge UI.
//
// Types supported (V1): `string`, `select`, `date`, `boolean`.
// The editor shape is:
//   - `boolean`  → checkbox
//   - `date`     → text input with NL-date resolution on save
//   - `select`   → text input (autocomplete from property keys in V2)
//   - `string`   → text input
//
// We deliberately keep all four cases on a single `<input type="text">`
// for V1 — Logseq/Notion both do the same until users ask for richer
// widgets. The type still controls the save transformation (NL-date
// resolution kicks in only for `date` keys, see the resolver hook).

import { useEffect, useRef, useState } from 'react'
import { api } from '@core/api-client'
import { resolveNaturalDate, isDatePropertyKey } from '@shared/utils/naturalDate'
import { getInlinePropertyKeys } from './propertyTemplate'
import type { Block, BlockProperty } from '@shared/types/api'

interface InlinePropertyBadgesProps {
  block: Block
  /** Called when a property value is successfully persisted. */
  onUpdate: (block: Block) => void
}

/**
 * Read a property value from the block, returning `null` for an
 * absent or empty value. The shape is always `string` for V1
 * badges — booleans and numbers get stringified for display.
 */
function readPropValue(block: Block, key: string): string | null {
  const prop = block.properties?.find(p => p.key === key)
  if (!prop || prop.value == null) return null
  return String(prop.value)
}

/**
 * Normalise a save value: run the natural-date resolver for `date`
 * keys, leave everything else as the user typed it.
 *
 * Why centralise: the panel and the badge both need this; the
 * resolver is a pure function so duplicating it in two places is
 * the kind of thing that drifts and ends in a regression.
 */
export function resolvePropertyValue(key: string, raw: string): string {
  if (isDatePropertyKey(key)) {
    const resolved = resolveNaturalDate(raw)
    if (resolved !== null) return resolved
  }
  return raw
}

/**
 * Build the next `properties` array after editing `key` → `value`.
 * Used by the optimistic local update so the parent sees the
 * new value immediately, then the API call follows.
 */
function withUpdatedProperty(
  block: Block,
  key: string,
  value: string | null,
): BlockProperty[] {
  const existing = block.properties ?? []
  const filtered = existing.filter(p => p.key !== key)
  if (value === null) return filtered
  // Look up the existing type so we can preserve it. If the property
  // is brand-new (filtered out + new) we default to `string`.
  const old = existing.find(p => p.key === key)
  const type: BlockProperty['type'] = old?.type ?? 'string'
  return [...filtered, { key, value, type }]
}

export function InlinePropertyBadges({ block, onUpdate }: InlinePropertyBadgesProps) {
  const inlineKeys = getInlinePropertyKeys(block)
  // `editingKey` is null when the user is not editing a badge. We
  // only allow one badge in edit mode at a time — opening a second
  // badge cancels the first (matches Notion behaviour).
  const [editingKey, setEditingKey] = useState<string | null>(null)
  // The current value of the inline editor. Initialised from the
  // property value when `editingKey` changes.
  const [editingValue, setEditingValue] = useState<string>('')
  // Ref to the active input so we can focus it on open.
  const inputRef = useRef<HTMLInputElement>(null)

  // Focus the input when the user opens a badge.
  useEffect(() => {
    if (editingKey && inputRef.current) {
      inputRef.current.focus()
      inputRef.current.select()
    }
  }, [editingKey])

  if (inlineKeys.length === 0) return null

  function openEditor(key: string) {
    const current = readPropValue(block, key) ?? ''
    setEditingKey(key)
    setEditingValue(current)
  }

  function closeEditor() {
    setEditingKey(null)
    setEditingValue('')
  }

  async function save(key: string) {
    const resolved = resolvePropertyValue(key, editingValue)
    // Skip the round-trip if the value did not actually change —
    // the click-to-open + click-away pattern would otherwise spam
    // the API.
    const previous = readPropValue(block, key) ?? ''
    if (resolved === previous) {
      closeEditor()
      return
    }

    // Optimistic local update so the badge text snaps to the new
    // value immediately. The parent is responsible for refreshing
    // from the server on the next sync tick.
    const nextProps = withUpdatedProperty(block, key, resolved)
    onUpdate({ ...block, properties: nextProps })
    closeEditor()

    try {
      await api.setBlockProperty(block.id, key, resolved)
    } catch {
      // Revert on failure. The next `getBlockProperties` reload
      // will also reconcile.
      onUpdate(block)
    }
  }

  return (
    <div
      data-testid="inline-property-badges"
      style={{ display: 'inline-flex', gap: '4px', alignItems: 'center', flexWrap: 'wrap' }}
    >
      {inlineKeys.map(key => {
        const isEditing = editingKey === key
        const value = readPropValue(block, key) ?? ''
        if (isEditing) {
          return (
            <input
              key={key}
              ref={inputRef}
              data-testid={`inline-editor-${key}`}
              value={editingValue}
              onChange={e => setEditingValue(e.target.value)}
              onBlur={() => save(key)}
              onKeyDown={e => {
                if (e.key === 'Enter') {
                  e.preventDefault()
                  save(key)
                }
                if (e.key === 'Escape') {
                  e.preventDefault()
                  closeEditor()
                }
              }}
              style={{
                fontSize: '11px',
                fontWeight: 600,
                padding: '2px 8px',
                borderRadius: 'var(--radius-pill)',
                border: '1px solid var(--color-accent)',
                background: 'var(--color-surface)',
                color: 'var(--color-text-primary)',
                outline: 'none',
                minWidth: '40px',
                maxWidth: '160px',
                fontFamily: 'inherit',
                lineHeight: 1.4,
              }}
            />
          )
        }
        return (
          <button
            key={key}
            type="button"
            data-testid={`inline-badge-${key}`}
            onClick={() => openEditor(key)}
            title={`${key}: ${value || '(empty)'} — click to edit`}
            style={{
              fontSize: '11px',
              fontWeight: 600,
              padding: '2px 8px',
              borderRadius: 'var(--radius-pill)',
              background: 'var(--color-surface-subtle)',
              color: 'var(--color-text-secondary)',
              border: '1px solid var(--color-border)',
              cursor: 'pointer',
              lineHeight: 1.4,
              letterSpacing: '0.01em',
              fontFamily: 'inherit',
            }}
          >
            {key}: {value || '—'}
          </button>
        )
      })}
    </div>
  )
}
