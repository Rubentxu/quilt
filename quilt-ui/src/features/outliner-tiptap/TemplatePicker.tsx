/**
 * TemplatePicker — ADR-0007
 *
 * EmptyState companion that replaces the 3 hardcoded "Add first
 * block" / "+ Reference" / "+ Documentation" buttons with a real
 * picker driven by the server's `quilt_list_templates` endpoint
 * (exposed at GET /api/v1/templates).
 *
 * Behavior:
 * - On mount, fetch the list of available templates
 * - If templates exist: show a search input + grid of cards, each
 *   with the template's icon + card-shape preview
 * - If no templates exist: fall back to a single "Add first block"
 *   button (so the user isn't stuck if they haven't created any
 *   `template/*` pages yet)
 * - If a template is selected, create a new block with
 *   `template:: <name>` and the title the user enters
 *
 * Design follows Notion's database-template picker UX: search bar
 * + visual cards. Cards are intentionally minimal — the actual
 * card rendering is data-driven (CardRenderer reads the
 * template's `card-shape::`).
 */

import { useState, useEffect, useCallback, useMemo } from 'react'
import { FileText, Search } from 'lucide-react'
import { api } from '@core/api-client'
import type { TemplateSummary, CardShape } from '@shared/types/api'

interface TemplatePickerProps {
  /** Called when the user picks a template (or the "Add first block" fallback). */
  onCreateBlock: (templateName: string | null, title: string) => void
  /** Whether the user is on a journal page (affects the default title). */
  isJournal?: boolean
}

export function TemplatePicker({ onCreateBlock, isJournal }: TemplatePickerProps) {
  const [templates, setTemplates] = useState<TemplateSummary[]>([])
  const [loading, setLoading] = useState(true)
  const [search, setSearch] = useState('')
  const [selected, setSelected] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)

  // Fetch the template list once per mount
  useEffect(() => {
    let cancelled = false
    api.listTemplates()
      .then(list => {
        if (cancelled) return
        setTemplates(list)
      })
      .catch(err => {
        if (cancelled) return
        // Non-fatal — fall back to "no templates" view
        // eslint-disable-next-line no-console
        console.warn('[TemplatePicker] Failed to load templates:', err)
        setError(err instanceof Error ? err.message : String(err))
      })
      .finally(() => {
        if (!cancelled) setLoading(false)
      })
    return () => { cancelled = true }
  }, [])

  const filtered = useMemo(() => {
    if (!search.trim()) return templates
    const q = search.toLowerCase()
    return templates.filter(t =>
      t.name.toLowerCase().includes(q) ||
      t.full_name.toLowerCase().includes(q) ||
      (t.icon && t.icon.includes(q))
    )
  }, [templates, search])

  const handleConfirm = useCallback(() => {
    if (selected) {
      onCreateBlock(selected, titleForTemplate(selected))
    } else {
      // Fallback: no template selected — just a plain block
      onCreateBlock(null, isJournal ? '' : '')
    }
  }, [selected, onCreateBlock, isJournal])

  // No templates yet — fall back to a simple "Add first block" button.
  // This matches the prior UX so the user isn't stuck before they
  // create any `template/*` pages. The picker becomes useful the
  // moment they create one.
  if (!loading && templates.length === 0) {
    return (
      <div data-testid="template-picker-empty">
        <button
          onClick={() => onCreateBlock(null, isJournal ? '' : '')}
          style={{
            padding: '8px 20px',
            fontSize: '14px',
            fontWeight: 500,
            background: 'var(--color-primary)',
            color: 'var(--color-on-primary, #fff)',
            border: 'none',
            borderRadius: 'var(--radius-md)',
            cursor: 'pointer',
          }}
        >
          Add first block
        </button>
        {error && (
          <p
            className="type-body-sm"
            style={{
              color: 'var(--color-text-muted)',
              marginTop: 'var(--space-2)',
              maxWidth: '320px',
            }}
          >
            Couldn't load templates ({error}). Create one at{' '}
            <code>template/&lt;name&gt;</code> to see it here.
          </p>
        )}
      </div>
    )
  }

  return (
    <div data-testid="template-picker" style={{ width: '100%', maxWidth: '640px', margin: '0 auto' }}>
      {/* Search bar */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 'var(--space-2)',
          padding: '8px 12px',
          background: 'var(--color-surface)',
          border: '1px solid var(--color-border)',
          borderRadius: 'var(--radius-md)',
          marginBottom: 'var(--space-3)',
        }}
      >
        <Search size={14} style={{ color: 'var(--color-text-muted)', flexShrink: 0 }} />
        <input
          type="text"
          placeholder="Pick a template…"
          value={search}
          onChange={e => setSearch(e.target.value)}
          autoFocus
          data-testid="template-picker-search"
          style={{
            flex: 1,
            background: 'transparent',
            border: 'none',
            outline: 'none',
            fontSize: '14px',
            color: 'var(--color-text-primary)',
            fontFamily: 'inherit',
          }}
        />
      </div>

      {/* Card grid */}
      {loading ? (
        <p
          className="type-body-sm"
          style={{ color: 'var(--color-text-muted)', textAlign: 'center', padding: 'var(--space-6) 0' }}
        >
          Loading templates…
        </p>
      ) : filtered.length === 0 ? (
        <p
          className="type-body-sm"
          style={{ color: 'var(--color-text-muted)', textAlign: 'center', padding: 'var(--space-6) 0' }}
        >
          No templates match "{search}"
        </p>
      ) : (
        <div
          style={{
            display: 'grid',
            gridTemplateColumns: 'repeat(auto-fill, minmax(180px, 1fr))',
            gap: 'var(--space-3)',
            marginBottom: 'var(--space-4)',
          }}
        >
          {filtered.map(t => (
            <TemplateCard
              key={t.name}
              template={t}
              selected={selected === t.name}
              onSelect={() => setSelected(t.name)}
            />
          ))}
        </div>
      )}

      {/* Confirm button */}
      <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 'var(--space-2)' }}>
        {selected && (
          <button
            onClick={() => setSelected(null)}
            style={{
              padding: '8px 16px',
              fontSize: '14px',
              fontWeight: 500,
              background: 'transparent',
              color: 'var(--color-text-secondary)',
              border: '1px solid var(--color-border)',
              borderRadius: 'var(--radius-md)',
              cursor: 'pointer',
            }}
          >
            Cancel
          </button>
        )}
        <button
          onClick={handleConfirm}
          data-testid="template-picker-confirm"
          style={{
            padding: '8px 20px',
            fontSize: '14px',
            fontWeight: 500,
            background: selected ? 'var(--color-primary)' : 'var(--color-surface-subtle)',
            color: selected ? 'var(--color-on-primary, #fff)' : 'var(--color-text-muted)',
            border: 'none',
            borderRadius: 'var(--radius-md)',
            cursor: selected ? 'pointer' : 'not-allowed',
          }}
          disabled={!selected}
        >
          {selected ? `Use ${selected}` : 'Pick a template'}
        </button>
      </div>
    </div>
  )
}

// ── TemplateCard (private) ─────────────────────────────────────

interface TemplateCardProps {
  template: TemplateSummary
  selected: boolean
  onSelect: () => void
}

function TemplateCard({ template, selected, onSelect }: TemplateCardProps) {
  const shape = template.card_shape as CardShape
  const shapeLabel = shape === 'reference' ? 'Reference' : shape === 'content' ? 'Documentation' : 'Inline'

  return (
    <button
      onClick={onSelect}
      data-testid={`template-card-${template.name}`}
      data-selected={selected}
      style={{
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'flex-start',
        gap: 'var(--space-2)',
        padding: 'var(--space-3)',
        background: 'var(--color-surface)',
        border: selected ? '2px solid var(--color-primary)' : '1px solid var(--color-border)',
        borderRadius: 'var(--radius-md)',
        cursor: 'pointer',
        textAlign: 'left',
        fontFamily: 'inherit',
        transition: 'border-color var(--motion-fast) var(--ease-standard), box-shadow var(--motion-fast) var(--ease-standard)',
        boxShadow: selected ? '0 0 0 1px var(--color-primary)' : 'none',
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-2)' }}>
        <span style={{ fontSize: '18px', lineHeight: 1 }}>
          {template.icon ?? <FileText size={16} />}
        </span>
        <span
          style={{
            fontSize: '13px',
            fontWeight: 600,
            color: 'var(--color-text-primary)',
            letterSpacing: '-0.01em',
          }}
        >
          {template.name}
        </span>
      </div>
      <span
        className="type-body-sm"
        style={{
          color: 'var(--color-text-muted)',
          fontSize: '11px',
        }}
      >
        {shapeLabel} · {template.block_count} block{template.block_count === 1 ? '' : 's'}
      </span>
    </button>
  )
}

// ── Helpers ─────────────────────────────────────────────────────

function titleForTemplate(templateName: string): string {
  // Pre-fill the new block's title with the template's display
  // name (capitalized). The user typically replaces it as they
  // fill the card's properties.
  return templateName
    .split(/[-_\/]/)
    .map(part => part.charAt(0).toUpperCase() + part.slice(1))
    .join(' ')
}
