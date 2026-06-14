/**
 * cellRenderers — typed cell renderers for TableView, keyed by PropertyType.
 *
 * Each renderer receives `(value, def)` and returns a React node. The map
 * lookup is type-safe against the `PropertyType` union; unknown types fall
 * back to the text renderer. This module is the single source of truth for
 * how a property value is *displayed* — the schema decides *how it's typed*.
 *
 * ─── Design rules ───────────────────────────────────────────────
 *
 * 1. Never throw on bad input. `null` / `undefined` / unknown shapes all
 *    render as `EmptyCell` (a thin em-dash) so the table stays consistent.
 * 2. No raw user input as JSX. All string values are coerced through
 *    `String(value)` so React's escaping handles XSS.
 * 3. Clickable cells (URLs) stop propagation so the row-click handler
 *    (when one is wired in a later batch) doesn't fire.
 * 4. Compact sizes (12–13px) match the TableView's row height; pill
 *    chips for select / multi_select carry the option's colour at 12%
 *    alpha for visual continuity with the rest of the UI.
 *
 * ─── Integration ────────────────────────────────────────────────
 *
 * `getCellRenderer(type)` returns the renderer for a `PropertyDef.type`.
 * The table dispatcher (SavedViewBlock) builds `ColumnDef.render` from
 * the renderer so TableView stays schema-agnostic — it just calls
 * `col.render(value, row)` when defined, else its built-in DefaultCell.
 */

import type { PropertyDef, PropertyOptions, SelectOptions } from '@shared/types/propertySchema'

// ─── Base cell renderer type ───────────────────────────────────────

export type CellRenderer = (
  value: unknown,
  def: PropertyDef,
) => React.ReactNode

// ─── Empty cell ───────────────────────────────────────────────────

function EmptyCell(): React.ReactNode {
  return (
    <span style={{ color: 'var(--color-text-disabled)', fontSize: '13px' }}>
      —
    </span>
  )
}

// ─── Text (default) ───────────────────────────────────────────────

function renderText(value: unknown, _def: PropertyDef): React.ReactNode {
  if (value == null || value === '') return <EmptyCell />
  return <span style={{ fontSize: '13px' }}>{String(value)}</span>
}

// ─── Select / multi_select ────────────────────────────────────────

function getSelectOptions(
  options: PropertyOptions | undefined,
): SelectOptions['options'] {
  if (
    options &&
    (options.type === 'select' || options.type === 'multi_select')
  ) {
    return options.options
  }
  return []
}

function renderSelect(value: unknown, def: PropertyDef): React.ReactNode {
  if (value == null) return <EmptyCell />
  const options = getSelectOptions(def.options)

  const values = Array.isArray(value) ? value : [value]
  return (
    <span
      style={{
        display: 'flex',
        gap: '4px',
        flexWrap: 'wrap',
        alignItems: 'center',
      }}
    >
      {values.map((v, i) => {
        const opt = options.find((o) => o.name === String(v))
        return (
          <span
            key={`${String(v)}-${i}`}
            style={{
              fontSize: '11px',
              fontWeight: 600,
              padding: '1px 8px',
              borderRadius: 'var(--radius-pill)',
              background: opt?.color
                ? `${opt.color}20`
                : 'var(--color-surface-subtle)',
              color: opt?.color ?? 'var(--color-text-primary)',
              whiteSpace: 'nowrap',
            }}
          >
            {String(v)}
          </span>
        )
      })}
    </span>
  )
}

// ─── Boolean ──────────────────────────────────────────────────────

function renderBoolean(value: unknown, _def: PropertyDef): React.ReactNode {
  if (value === true) {
    return (
      <span
        aria-label="Yes"
        title="Yes"
        style={{ color: 'var(--color-accent)', fontSize: '13px' }}
      >
        ☑
      </span>
    )
  }
  if (value === false) {
    return (
      <span
        aria-label="No"
        title="No"
        style={{ color: 'var(--color-text-muted)', fontSize: '13px' }}
      >
        ☐
      </span>
    )
  }
  return <EmptyCell />
}

// ─── Date ─────────────────────────────────────────────────────────

function renderDate(value: unknown, def: PropertyDef): React.ReactNode {
  if (value == null) return <EmptyCell />
  const dateStr = String(value)
  const format =
    def.options?.type === 'date' ? def.options.format : undefined

  let display = dateStr
  try {
    const d = new Date(dateStr)
    if (!isNaN(d.getTime())) {
      if (format === 'relative') {
        display = d.toLocaleDateString('en-US', {
          weekday: 'short',
          month: 'short',
          day: 'numeric',
        })
      } else {
        display = d.toLocaleDateString('en-US', {
          year: 'numeric',
          month: 'short',
          day: 'numeric',
        })
      }
    }
  } catch {
    // Use raw string fallback
  }

  return <span style={{ fontSize: '13px' }}>{display}</span>
}

// ─── URL ──────────────────────────────────────────────────────────

function renderUrl(value: unknown, _def: PropertyDef): React.ReactNode {
  if (value == null) return <EmptyCell />
  const url = String(value)
  return (
    <a
      href={url}
      target="_blank"
      rel="noopener noreferrer"
      data-testid="cell-url"
      style={{
        fontSize: '13px',
        color: 'var(--color-accent)',
        textDecoration: 'underline',
      }}
      onClick={(e) => e.stopPropagation()}
    >
      {url.length > 40 ? `${url.slice(0, 40)}…` : url}
    </a>
  )
}

// ─── Number ───────────────────────────────────────────────────────

function renderNumber(value: unknown, def: PropertyDef): React.ReactNode {
  if (value == null) return <EmptyCell />
  const num = Number(value)
  if (isNaN(num)) return <span style={{ fontSize: '13px' }}>{String(value)}</span>

  const fmt = def.options?.type === 'number' ? def.options.format : undefined
  let display: string
  if (fmt === 'percent') {
    display = `${(num * 100).toFixed(1)}%`
  } else if (fmt === 'currency') {
    const curr = def.options?.type === 'number' ? def.options.currency : '$'
    display = `${curr}${num.toLocaleString()}`
  } else {
    display = num.toLocaleString()
  }

  return (
    <span
      style={{
        fontSize: '13px',
        fontVariantNumeric: 'tabular-nums',
      }}
    >
      {display}
    </span>
  )
}

// ─── Person ───────────────────────────────────────────────────────

function renderPerson(value: unknown, _def: PropertyDef): React.ReactNode {
  if (value == null) return <EmptyCell />
  const name = String(value)
  return (
    <span
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: '6px',
        fontSize: '13px',
      }}
    >
      <span
        aria-hidden="true"
        style={{
          width: '20px',
          height: '20px',
          borderRadius: '50%',
          background: 'var(--color-accent-subtle)',
          color: 'var(--color-accent)',
          display: 'inline-flex',
          alignItems: 'center',
          justifyContent: 'center',
          fontSize: '10px',
          fontWeight: 600,
          flexShrink: 0,
        }}
      >
        {name.charAt(0).toUpperCase()}
      </span>
      <span>{name}</span>
    </span>
  )
}

// ─── Relation ─────────────────────────────────────────────────────

function renderRelation(value: unknown, _def: PropertyDef): React.ReactNode {
  if (value == null) return <EmptyCell />
  const target = String(value)
  return (
    <span
      style={{
        fontSize: '13px',
        color: 'var(--color-accent)',
      }}
    >
      → {target}
    </span>
  )
}

// ─── File ─────────────────────────────────────────────────────────

function renderFile(value: unknown, _def: PropertyDef): React.ReactNode {
  if (value == null) return <EmptyCell />
  const name = String(value)
  return (
    <span
      style={{
        fontSize: '13px',
        display: 'inline-flex',
        alignItems: 'center',
        gap: '4px',
      }}
    >
      <span aria-hidden="true">📎</span>
      <span>{name.length > 30 ? `${name.slice(0, 30)}…` : name}</span>
    </span>
  )
}

// ─── Public map ───────────────────────────────────────────────────

/**
 * Renderer lookup keyed by `PropertyType`. Multi-select reuses the select
 * renderer since the only difference is whether `value` is a string or an
 * array of strings — `renderSelect` normalises both.
 */
export const CELL_RENDERERS: Record<string, CellRenderer> = {
  text: renderText,
  number: renderNumber,
  select: renderSelect,
  multi_select: renderSelect,
  date: renderDate,
  boolean: renderBoolean,
  url: renderUrl,
  person: renderPerson,
  relation: renderRelation,
  file: renderFile,
}

/**
 * Get the appropriate cell renderer for a property type.
 * Falls back to `renderText` for unknown types so a typo in the schema
 * never crashes the table.
 */
export function getCellRenderer(type: string): CellRenderer {
  return CELL_RENDERERS[type] ?? renderText
}
