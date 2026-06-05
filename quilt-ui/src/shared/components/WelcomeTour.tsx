// ─── WelcomeTour — first-run product tour (F3) ────────────────────
//
// Modal that appears the first time a user lands on Quilt. Explains
// the four key primitives in 4 cards:
//
//   1. Plantillas — sidebar templates
//   2. Recientes — sidebar recents
//   3. Slash command — `/` at the start of a block
//   4. Properties — typed block properties
//
// The "seen" state is persisted to `STORAGE_KEYS.WELCOME_SEEN`
// (`'quilt-welcome-seen'`). Once dismissed, the modal does not
// re-appear unless the user manually clears the flag. Mounting the
// component is a no-op when the flag is already set — `AppShell`
// gates on it and never re-renders the dialog.
//
// Spec: F3 of `quilt-fase2-ux-empty-states`. Single PR, no
// chained work. Design follows DESIGN.md §9.10 / §15 (empty
// state principles applied to a "what now?" first-run state).

import { useEffect, useRef } from 'react'
import { createPortal } from 'react-dom'
import { FileText, Clock, Terminal, Settings, X, Sparkles } from 'lucide-react'
import { STORAGE_KEYS } from '@features/sidebar/storage-keys'

interface WelcomeTourProps {
  /** Called once the user dismisses the tour. */
  onClose: () => void
}

const cardBaseStyle: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  alignItems: 'flex-start',
  gap: 'var(--space-2)',
  padding: 'var(--space-3)',
  background: 'var(--color-surface)',
  border: '1px solid var(--color-border)',
  borderRadius: 'var(--radius-md)',
  textAlign: 'left',
}

export function WelcomeTour({ onClose }: WelcomeTourProps) {
  // Escape-to-close (matches the BlockContextMenu + TopbarMenu
  // dismiss behaviour from prior PRs so users only have to learn
  // it once).
  const dialogRef = useRef<HTMLDivElement>(null)
  const closeButtonRef = useRef<HTMLButtonElement>(null)

  useEffect(() => {
    function handleKey(e: KeyboardEvent) {
      if (e.key === 'Escape') {
        e.preventDefault()
        handleClose()
      }
    }
    document.addEventListener('keydown', handleKey)
    // Auto-focus the close button on mount so keyboard users can
    // dismiss the dialog without tabbing through the cards.
    closeButtonRef.current?.focus()
    return () => document.removeEventListener('keydown', handleKey)
    // handleClose is stable (it depends on `onClose` and the
    // localStorage shim, both of which are stable for the
    // lifetime of the dialog). Including it would re-install the
    // listener on every render and steal focus from the close
    // button after the user has tabbed into the cards.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [onClose])

  function handleClose() {
    // Persist before unmounting so a fast back-nav doesn't trigger
    // the tour twice.
    try {
      localStorage.setItem(STORAGE_KEYS.WELCOME_SEEN, '1')
    } catch {
      // localStorage may be unavailable (private mode, quota). The
      // in-memory state still hides the tour for the rest of the
      // session — the user can re-clear by reloading without the
      // flag.
    }
    onClose()
  }

  // Backdrop click also dismisses (matches most product-tour
  // conventions). We intentionally do NOT use a click outside the
  // dialog because the cards are inside the same wrapper as the
  // backdrop.
  function handleBackdropClick(e: React.MouseEvent<HTMLDivElement>) {
    if (e.target === e.currentTarget) {
      handleClose()
    }
  }

  const dialog = (
    <div
      role="presentation"
      onClick={handleBackdropClick}
      style={{
        position: 'fixed',
        inset: 0,
        background: 'rgba(15, 23, 42, 0.45)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        zIndex: 200,
        padding: 'var(--space-4)',
      }}
    >
      <div
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby="welcome-tour-title"
        data-testid="welcome-tour"
        style={{
          position: 'relative',
          width: '100%',
          maxWidth: '640px',
          maxHeight: 'calc(100vh - var(--space-8))',
          overflow: 'auto',
          background: 'var(--color-surface-elevated)',
          border: '1px solid var(--color-border)',
          borderRadius: 'var(--radius-lg)',
          boxShadow: 'var(--shadow-lg)',
          padding: 'var(--space-6)',
          display: 'flex',
          flexDirection: 'column',
          gap: 'var(--space-4)',
        }}
      >
        {/* Header */}
        <div
          style={{
            display: 'flex',
            alignItems: 'flex-start',
            justifyContent: 'space-between',
            gap: 'var(--space-3)',
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-2)' }}>
            <span
              aria-hidden="true"
              style={{
                width: '32px',
                height: '32px',
                borderRadius: '50%',
                background: 'var(--color-primary-container)',
                color: 'var(--color-primary)',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
              }}
            >
              <Sparkles size={16} />
            </span>
            <h2
              id="welcome-tour-title"
              className="type-title-md"
              style={{ margin: 0 }}
            >
              Welcome to Quilt
            </h2>
          </div>
          <button
            ref={closeButtonRef}
            type="button"
            onClick={handleClose}
            aria-label="Close welcome tour"
            data-testid="welcome-tour-close"
            className="ghost-icon-button"
            style={{
              background: 'transparent',
              border: 'none',
              cursor: 'pointer',
              color: 'var(--color-text-muted)',
              padding: 'var(--space-1)',
              borderRadius: 'var(--radius-sm)',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
            }}
          >
            <X size={18} />
          </button>
        </div>

        <p
          className="type-body"
          style={{ margin: 0, color: 'var(--color-text-secondary)' }}
        >
          Four things to know before you start writing.
        </p>

        {/* Four feature cards */}
        <div
          style={{
            display: 'grid',
            gridTemplateColumns: 'repeat(auto-fit, minmax(220px, 1fr))',
            gap: 'var(--space-3)',
          }}
        >
          <FeatureCard
            icon={<FileText size={16} aria-hidden="true" />}
            title="Plantillas"
            body="Click any template in the sidebar to start a new page from a pre-filled structure (Reference, Documentation, etc.)."
          />
          <FeatureCard
            icon={<Clock size={16} aria-hidden="true" />}
            title="Recientes"
            body="Pages you visit appear under “Recientes” in the sidebar — one click back to where you were."
          />
          <FeatureCard
            icon={<Terminal size={16} aria-hidden="true" />}
            title="Slash command"
            body="Type / at the start of a block to open the command menu — insert templates, transform to-do, toggle properties, and more."
          />
          <FeatureCard
            icon={<Settings size={16} aria-hidden="true" />}
            title="Properties"
            body="Right-click any block to add typed properties. Properties power Kanban views, filters and the graph."
          />
        </div>

        {/* Footer CTA */}
        <div
          style={{
            display: 'flex',
            justifyContent: 'flex-end',
            gap: 'var(--space-2)',
            borderTop: '1px solid var(--color-border)',
            paddingTop: 'var(--space-3)',
          }}
        >
          <button
            type="button"
            onClick={handleClose}
            data-testid="welcome-tour-got-it"
            style={{
              padding: '8px 20px',
              fontSize: '13px',
              fontWeight: 600,
              background: 'var(--color-primary)',
              color: 'var(--color-on-primary, #fff)',
              border: 'none',
              borderRadius: 'var(--radius-md)',
              cursor: 'pointer',
              fontFamily: 'inherit',
            }}
            className="btn-primary"
          >
            Got it
          </button>
        </div>
      </div>
    </div>
  )

  // Portal so the tour is never trapped inside a parent with
  // `overflow: hidden` (the AppShell main pane is scrollable).
  return createPortal(dialog, document.body)
}

// ── FeatureCard (private) ─────────────────────────────────────

interface FeatureCardProps {
  icon: React.ReactNode
  title: string
  body: string
}

function FeatureCard({ icon, title, body }: FeatureCardProps) {
  return (
    <div style={cardBaseStyle} data-testid={`welcome-tour-card-${title.toLowerCase()}`}>
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 'var(--space-2)',
          color: 'var(--color-primary)',
        }}
      >
        {icon}
        <span
          style={{
            fontSize: '13px',
            fontWeight: 600,
            color: 'var(--color-text-primary)',
          }}
        >
          {title}
        </span>
      </div>
      <p
        className="type-body-sm"
        style={{
          margin: 0,
          color: 'var(--color-text-secondary)',
          lineHeight: 1.45,
        }}
      >
        {body}
      </p>
    </div>
  )
}
