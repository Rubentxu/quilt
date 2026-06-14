/**
 * SortChips — visual editor for a view's sort configuration.
 *
 * Renders one chip per active sort directive, plus a "+ Sort" affordance
 * that opens a `<select>` dropdown of *unused* property keys. The parent
 * owns the sort state and is notified via `onChange` for every mutation
 * (add, remove, toggle direction). No persistence happens here — that's
 * the dispatcher's job (Batch 10 wires the explicit save-on-edit).
 *
 * ─── Test hooks ──────────────────────────────────────────────────
 *
 *   data-testid="sort-chips"            — wrapper
 *   data-testid="sort-chip-{idx}"        — one chip per active sort
 *   data-testid="add-sort-button"       — the "+ Sort" button
 *   data-testid="add-sort-select"       — the dropdown when open
 *
 * The select uses native semantics (no custom popover) so keyboard,
 * screen reader, and mobile pickers work out of the box, and the test
 * surface stays small.
 */

import { useState } from 'react'
import type { ViewSort } from '@shared/types/viewConfig'

export interface SortChipsProps {
  /** Current sort state from viewConfig. */
  sorts: ViewSort[]
  /** Available property keys to sort by. */
  availableKeys: string[]
  /** Called when sorts change. */
  onChange: (sorts: ViewSort[]) => void
}

export function SortChips({ sorts, availableKeys, onChange }: SortChipsProps) {
  const [showAdd, setShowAdd] = useState(false)

  const addSort = (key: string) => {
    onChange([...sorts, { propertyKey: key, direction: 'asc' }])
    setShowAdd(false)
  }

  const removeSort = (idx: number) => {
    onChange(sorts.filter((_, i) => i !== idx))
  }

  const toggleDirection = (idx: number) => {
    const updated = sorts.map((s, i) =>
      i === idx
        ? { ...s, direction: s.direction === 'asc' ? ('desc' as const) : ('asc' as const) }
        : s,
    )
    onChange(updated)
  }

  const unaddable = availableKeys.filter(
    (k) => !sorts.some((s) => s.propertyKey === k),
  )

  return (
    <div
      data-testid="sort-chips"
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: '6px',
        flexWrap: 'wrap',
        padding: '6px 0',
      }}
    >
      {sorts.map((sort, idx) => (
        <span
          key={`${sort.propertyKey}-${idx}`}
          data-testid={`sort-chip-${idx}`}
          style={{
            display: 'inline-flex',
            alignItems: 'center',
            gap: '4px',
            padding: '2px 8px',
            borderRadius: 'var(--radius-pill)',
            background: 'var(--color-surface-subtle)',
            border: '1px solid var(--color-border)',
            fontSize: '12px',
            color: 'var(--color-text-primary)',
          }}
        >
          <button
            type="button"
            onClick={() => toggleDirection(idx)}
            title={`Toggle direction (currently ${sort.direction})`}
            aria-label={`Toggle direction for ${sort.propertyKey}`}
            data-testid={`sort-direction-${idx}`}
            style={{
              background: 'none',
              border: 'none',
              cursor: 'pointer',
              padding: 0,
              fontSize: '10px',
              color: 'var(--color-text-primary)',
            }}
          >
            {sort.direction === 'asc' ? '↑' : '↓'}
          </button>
          <span>{sort.propertyKey}</span>
          <button
            type="button"
            onClick={() => removeSort(idx)}
            title="Remove sort"
            aria-label={`Remove sort by ${sort.propertyKey}`}
            data-testid={`sort-remove-${idx}`}
            style={{
              background: 'none',
              border: 'none',
              cursor: 'pointer',
              padding: 0,
              fontSize: '12px',
              lineHeight: 1,
              color: 'var(--color-text-muted)',
            }}
          >
            ×
          </button>
        </span>
      ))}

      {showAdd && unaddable.length > 0 ? (
        <div style={{ position: 'relative' }}>
          <select
            data-testid="add-sort-select"
            autoFocus
            defaultValue=""
            onChange={(e) => {
              if (e.target.value) addSort(e.target.value)
            }}
            onBlur={() => setShowAdd(false)}
            style={{
              padding: '2px 8px',
              borderRadius: 'var(--radius-pill)',
              border: '1px solid var(--color-accent)',
              fontSize: '12px',
              background: 'var(--color-surface)',
              color: 'var(--color-text-primary)',
            }}
          >
            <option value="" disabled>
              Add sort…
            </option>
            {unaddable.map((k) => (
              <option key={k} value={k}>
                {k}
              </option>
            ))}
          </select>
        </div>
      ) : (
        <button
          type="button"
          onClick={() => setShowAdd(true)}
          data-testid="add-sort-button"
          disabled={unaddable.length === 0}
          title={unaddable.length === 0 ? 'All available keys are already sorted' : 'Add a sort'}
          style={{
            background: 'none',
            border: '1px dashed var(--color-border)',
            borderRadius: 'var(--radius-pill)',
            padding: '2px 8px',
            fontSize: '12px',
            color: 'var(--color-text-muted)',
            cursor: unaddable.length === 0 ? 'not-allowed' : 'pointer',
            opacity: unaddable.length === 0 ? 0.5 : 1,
          }}
        >
          + Sort
        </button>
      )}
    </div>
  )
}
