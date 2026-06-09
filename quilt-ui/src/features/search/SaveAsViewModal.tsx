/**
 * SaveAsViewModal — ROADMAP #25 ("Save as View" desde search).
 *
 * Small presentational form for the "Save as View" flow that runs
 * inside `SearchModal`. The user picked a search result (page or
 * block) and chose a view-type + view-name + target page. This
 * component collects those three values and fires `onConfirm` with
 * the shape:
 *
 *     { name: string; viewType: ViewType; pageName: string }
 *
 * The caller (SearchModal) is responsible for:
 *   - building the search DSL from the parsed query
 *   - creating the `type:: query` block carrying the DSL
 *   - reading back the new block's UUID
 *   - creating the `type:: view` block with `data-source:: <uuid>`
 *
 * Keeping this component dumb on purpose: it has no business with
 * the API, with the query parser, or with the saved-view dispatcher.
 * It just renders the form. The split keeps the integration tests
 * focused on the form contract, and the SearchModal tests focused
 * on the wiring.
 *
 * ──── View types ──────────────────────────────────────────────────
 *
 * The view-type dropdown enumerates the same `VIEW_TYPES` set the
 * `SavedViewBlock` dispatcher understands (see
 * `quilt-ui/src/features/view/SavedViewBlock.tsx`). We re-declare
 * the list here to keep the modal self-contained: a SavedView with
 * an unknown view-type renders an error state, so the user would
 * just see a broken view if we let them pick a value the dispatcher
 * doesn't recognise. The "table" option is the first one (and
 * therefore the default) because it's the most common pick for a
 * search results view.
 */

import { useState } from 'react'
import type { Page } from '@shared/types/api'

/** Recognised view types. MUST stay in sync with SavedViewBlock.VIEW_TYPES. */
const VIEW_TYPES = [
  'table',
  'kanban',
  'calendar',
  'list',
  'graph',
  'cards',
  'timeline',
] as const

export type SaveAsViewType = (typeof VIEW_TYPES)[number]

export interface SaveAsViewRequest {
  name: string
  viewType: SaveAsViewType
  pageName: string
}

export interface SaveAsViewModalProps {
  /** All pages in the graph. Rendered as <option>s in the page selector. */
  pages: Page[]
  /** Disables the submit button while the createBlock calls are in flight. */
  isSubmitting: boolean
  /** When set, an error banner is shown above the buttons. */
  errorMessage: string | null
  onConfirm: (req: SaveAsViewRequest) => void
  onCancel: () => void
}

export function SaveAsViewModal({
  pages,
  isSubmitting,
  errorMessage,
  onConfirm,
  onCancel,
}: SaveAsViewModalProps) {
  const [name, setName] = useState('')
  const [viewType, setViewType] = useState<SaveAsViewType>('table')
  const [pageName, setPageName] = useState(pages[0]?.name ?? '')

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault()
    const trimmed = name.trim()
    if (!trimmed || !pageName) return
    onConfirm({ name: trimmed, viewType, pageName })
  }

  return (
    <div
      data-testid="save-view-modal"
      role="dialog"
      aria-label="Save as view"
      onClick={e => e.stopPropagation()}
      style={{
        position: 'fixed',
        inset: 0,
        zIndex: 110, // above SearchModal (100)
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        background: 'rgba(0, 0, 0, 0.45)',
      }}
    >
      <form
        onSubmit={handleSubmit}
        style={{
          width: '100%',
          maxWidth: '440px',
          background: 'var(--color-surface)',
          borderRadius: 'var(--radius-lg)',
          boxShadow: 'var(--shadow-lg)',
          padding: 'var(--space-4)',
          display: 'flex',
          flexDirection: 'column',
          gap: 'var(--space-3)',
        }}
      >
        <h2
          style={{
            margin: 0,
            fontSize: '16px',
            fontWeight: 600,
            color: 'var(--color-text-primary)',
          }}
        >
          Save as view
        </h2>

        {/* View name */}
        <label
          style={{
            display: 'flex',
            flexDirection: 'column',
            gap: 'var(--space-1)',
            fontSize: '12px',
            fontWeight: 600,
            color: 'var(--color-text-muted)',
            textTransform: 'uppercase',
            letterSpacing: '0.04em',
          }}
        >
          View name
          <input
            data-testid="save-view-name-input"
            type="text"
            value={name}
            onChange={e => setName(e.target.value)}
            placeholder="e.g. Open tasks"
            autoFocus
            style={{
              padding: 'var(--space-2) var(--space-3)',
              border: '1px solid var(--color-border)',
              borderRadius: 'var(--radius-sm)',
              fontSize: '14px',
              background: 'var(--color-surface)',
              color: 'var(--color-text-primary)',
              fontWeight: 400,
              textTransform: 'none',
              letterSpacing: 'normal',
            }}
          />
        </label>

        {/* View type */}
        <label
          style={{
            display: 'flex',
            flexDirection: 'column',
            gap: 'var(--space-1)',
            fontSize: '12px',
            fontWeight: 600,
            color: 'var(--color-text-muted)',
            textTransform: 'uppercase',
            letterSpacing: '0.04em',
          }}
        >
          View type
          <select
            data-testid="save-view-type-select"
            value={viewType}
            onChange={e => setViewType(e.target.value as SaveAsViewType)}
            style={{
              padding: 'var(--space-2) var(--space-3)',
              border: '1px solid var(--color-border)',
              borderRadius: 'var(--radius-sm)',
              fontSize: '14px',
              background: 'var(--color-surface)',
              color: 'var(--color-text-primary)',
              fontWeight: 400,
              textTransform: 'none',
              letterSpacing: 'normal',
            }}
          >
            {VIEW_TYPES.map(vt => (
              <option key={vt} value={vt}>
                {vt}
              </option>
            ))}
          </select>
        </label>

        {/* Page selector */}
        <label
          style={{
            display: 'flex',
            flexDirection: 'column',
            gap: 'var(--space-1)',
            fontSize: '12px',
            fontWeight: 600,
            color: 'var(--color-text-muted)',
            textTransform: 'uppercase',
            letterSpacing: '0.04em',
          }}
        >
          Save in page
          <select
            data-testid="save-view-page-select"
            value={pageName}
            onChange={e => setPageName(e.target.value)}
            style={{
              padding: 'var(--space-2) var(--space-3)',
              border: '1px solid var(--color-border)',
              borderRadius: 'var(--radius-sm)',
              fontSize: '14px',
              background: 'var(--color-surface)',
              color: 'var(--color-text-primary)',
              fontWeight: 400,
              textTransform: 'none',
              letterSpacing: 'normal',
            }}
          >
            {pages.map(p => (
              <option key={p.id} value={p.name}>
                {p.name}
              </option>
            ))}
          </select>
        </label>

        {errorMessage && (
          <div
            data-testid="save-view-error"
            role="alert"
            style={{
              padding: 'var(--space-2) var(--space-3)',
              background: 'var(--color-danger-subtle, rgba(220, 38, 38, 0.08))',
              color: 'var(--color-danger, #dc2626)',
              borderRadius: 'var(--radius-sm)',
              fontSize: '13px',
            }}
          >
            {errorMessage}
          </div>
        )}

        <div
          style={{
            display: 'flex',
            justifyContent: 'flex-end',
            gap: 'var(--space-2)',
            marginTop: 'var(--space-1)',
          }}
        >
          <button
            type="button"
            data-testid="save-view-cancel"
            onClick={onCancel}
            disabled={isSubmitting}
            style={{
              padding: 'var(--space-2) var(--space-3)',
              background: 'transparent',
              border: '1px solid var(--color-border)',
              borderRadius: 'var(--radius-sm)',
              fontSize: '14px',
              color: 'var(--color-text-primary)',
              cursor: isSubmitting ? 'not-allowed' : 'pointer',
            }}
          >
            Cancel
          </button>
          <button
            type="submit"
            data-testid="save-view-submit"
            disabled={isSubmitting || !name.trim() || !pageName}
            style={{
              padding: 'var(--space-2) var(--space-3)',
              background: 'var(--color-accent)',
              border: '1px solid var(--color-accent)',
              borderRadius: 'var(--radius-sm)',
              fontSize: '14px',
              fontWeight: 600,
              color: 'var(--color-surface, #fff)',
              cursor:
                isSubmitting || !name.trim() || !pageName
                  ? 'not-allowed'
                  : 'pointer',
              opacity: isSubmitting || !name.trim() || !pageName ? 0.6 : 1,
            }}
          >
            {isSubmitting ? 'Saving…' : 'Save view'}
          </button>
        </div>
      </form>
    </div>
  )
}
