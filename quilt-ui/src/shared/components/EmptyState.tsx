/**
 * EmptyState — DESIGN.md §15
 *
 * Toda vista sin contenido debe mostrar:
 *   - Icono ilustrativo discreto
 *   - Título claro
 *   - Descripción breve
 *   - Acción principal
 *
 * El estado vacío nunca debe verse como un error. Es una oportunidad
 * para guiar al usuario a la primera acción útil.
 */

import { type ReactNode } from 'react'
import { Inbox } from 'lucide-react'

interface EmptyStateProps {
  /** Lucide icon component (or any React component) — falls back to Inbox. */
  icon?: ReactNode
  title: string
  description?: string
  action?: ReactNode
  /** Adds more vertical breathing room — use for full-page empty states. */
  fullPage?: boolean
}

export function EmptyState({
  icon,
  title,
  description,
  action,
  fullPage = false,
}: EmptyStateProps) {
  return (
    <div
      role="status"
      aria-live="polite"
      style={{
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        gap: 'var(--space-3)',
        padding: fullPage ? 'var(--space-12) var(--space-4)' : 'var(--space-8) var(--space-4)',
        textAlign: 'center',
      }}
    >
      <div
        style={{
          width: '48px',
          height: '48px',
          borderRadius: '50%',
          background: 'var(--color-surface-subtle)',
          color: 'var(--color-text-muted)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
        }}
      >
        {icon ?? <Inbox size={24} aria-hidden="true" />}
      </div>

      <h3
        style={{
          fontSize: '16px',
          fontWeight: 600,
          color: 'var(--color-text-primary)',
          margin: 0,
        }}
      >
        {title}
      </h3>

      {description && (
        <p
          style={{
            fontSize: '13px',
            color: 'var(--color-text-muted)',
            maxWidth: '400px',
            margin: 0,
            lineHeight: 1.5,
          }}
        >
          {description}
        </p>
      )}

      {action && (
        <div style={{ marginTop: 'var(--space-2)' }}>{action}</div>
      )}
    </div>
  )
}
