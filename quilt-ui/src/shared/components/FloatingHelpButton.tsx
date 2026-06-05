/**
 * FloatingHelpButton — DESIGN.md §9.10
 *
 * Botón flotante de ayuda.
 * Normas:
 *   - Posición: esquina inferior derecha
 *   - Accesible por teclado (focus ring)
 *   - Tooltip / aria-label
 *   - No tapa contenido importante
 *   - Respeta safe area en pantallas pequeñas
 */

import { useState, type ReactNode } from 'react'
import { HelpCircle, X } from 'lucide-react'

interface FloatingHelpButtonProps {
  /** Click handler — typically opens a modal or panel. */
  onClick?: () => void
  /** If provided, the icon swaps to this when expanded (e.g. close icon). */
  label?: string
  /** Optional panel to render when expanded (controlled by internal state). */
  panel?: ReactNode
  /**
   * When provided, the open/closed state becomes controlled by the
   * parent. The button still updates via `onExpandedChange`. Useful
   * for letting other parts of the UI (a top-bar kebab menu, a
   * keyboard shortcut) toggle the help panel.
   */
  expanded?: boolean
  onExpandedChange?: (next: boolean) => void
}

export function FloatingHelpButton({
  onClick,
  label = 'Help & shortcuts',
  panel,
  expanded: expandedProp,
  onExpandedChange,
}: FloatingHelpButtonProps) {
  const [internalExpanded, setInternalExpanded] = useState(false)
  // Controlled vs uncontrolled: prefer the prop when given, otherwise
  // fall back to local state. This keeps the existing call sites
  // (which never set `expanded`) working without any change.
  const isControlled = expandedProp !== undefined
  const expanded = isControlled ? expandedProp : internalExpanded
  const setExpanded = (next: boolean) => {
    if (isControlled) {
      onExpandedChange?.(next)
    } else {
      setInternalExpanded(next)
    }
  }

  function handleClick() {
    if (panel) {
      setExpanded(!expanded)
    } else {
      onClick?.()
    }
  }

  return (
    <>
      <button
        onClick={handleClick}
        aria-label={label}
        title={label}
        aria-expanded={expanded}
        style={{
          position: 'fixed',
          bottom: 'calc(var(--space-4) + env(safe-area-inset-bottom, 0px))',
          right: 'calc(var(--space-4) + env(safe-area-inset-right, 0px))',
          width: '44px',
          height: '44px',
          borderRadius: '50%',
          background: 'var(--color-surface-elevated)',
          color: 'var(--color-primary)',
          border: '1px solid var(--color-border)',
          boxShadow: 'var(--shadow-md)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          cursor: 'pointer',
          zIndex: 100,
          transition:
            'transform var(--motion-fast) var(--ease-standard), background var(--motion-fast) var(--ease-standard)',
        }}
        onMouseEnter={(e) => {
          e.currentTarget.style.background = 'var(--color-primary-container)'
        }}
        onMouseLeave={(e) => {
          e.currentTarget.style.background = 'var(--color-surface-elevated)'
        }}
      >
        {expanded ? <X size={20} aria-hidden="true" /> : <HelpCircle size={20} aria-hidden="true" />}
      </button>

      {expanded && panel && (
        <div
          role="dialog"
          aria-label="Help"
          style={{
            position: 'fixed',
            bottom: 'calc(var(--space-4) + 52px + env(safe-area-inset-bottom, 0px))',
            right: 'calc(var(--space-4) + env(safe-area-inset-right, 0px))',
            width: '320px',
            maxWidth: 'calc(100vw - var(--space-6))',
            background: 'var(--color-surface-elevated)',
            border: '1px solid var(--color-border)',
            borderRadius: 'var(--radius-lg)',
            boxShadow: 'var(--shadow-lg)',
            padding: 'var(--space-4)',
            zIndex: 99,
          }}
        >
          {panel}
        </div>
      )}
    </>
  )
}
