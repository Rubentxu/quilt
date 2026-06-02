/**
 * ContentCard — DESIGN.md §9.8
 *
 * Tarjeta para bloques largos de documentación dentro del contenido.
 * Se usa dentro de un bloque de página cuando el bloque incluye
 * documentación estructurada (resúmenes, listas, checklists, etc.).
 */
import { useState, type ReactNode } from 'react'
import { FileText, ChevronDown, ChevronRight, Plus } from 'lucide-react'

interface ContentCardProps {
  title: string
  subtitle?: string
  children: ReactNode
  defaultCollapsed?: boolean
  onAddItem?: () => void
}

export function ContentCard({
  title,
  subtitle,
  children,
  defaultCollapsed = false,
  onAddItem,
}: ContentCardProps) {
  const [collapsed, setCollapsed] = useState(defaultCollapsed)

  return (
    <div
      data-testid="content-card"
      style={{
        background: 'var(--color-surface)',
        border: '1px solid var(--color-border)',
        borderRadius: 'var(--radius-lg)',
        boxShadow: 'var(--shadow-sm)',
        padding: '0',
        margin: 'var(--space-3) 0',
        overflow: 'hidden',
      }}
    >
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 'var(--space-2)',
          padding: '12px 16px',
          borderBottom: collapsed ? 'none' : '1px solid var(--color-border)',
        }}
      >
        <button
          type="button"
          aria-label={collapsed ? 'Expand section' : 'Collapse section'}
          onClick={() => setCollapsed((v) => !v)}
          className="ghost-icon-button"
          style={{
            width: '28px',
            height: '28px',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            cursor: 'pointer',
            color: 'var(--color-text-secondary)',
          }}
        >
          {collapsed ? <ChevronRight size={15} /> : <ChevronDown size={15} />}
        </button>
        <FileText
          size={15}
          style={{ color: 'var(--color-text-muted)', flexShrink: 0 }}
        />
        <div style={{ flex: 1, minWidth: 0 }}>
          <div
            style={{
              fontSize: '14px',
              fontWeight: 600,
              color: 'var(--color-text-primary)',
              letterSpacing: '-0.01em',
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
            }}
          >
            {title}
          </div>
          {subtitle && (
            <div
              style={{
                fontSize: '12px',
                color: 'var(--color-text-muted)',
                marginTop: '2px',
              }}
            >
              {subtitle}
            </div>
          )}
        </div>
        {onAddItem && (
          <button
            type="button"
            aria-label="Add item"
            onClick={onAddItem}
            className="ghost-icon-button"
            style={{
              width: '28px',
              height: '28px',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              cursor: 'pointer',
              color: 'var(--color-text-secondary)',
            }}
          >
            <Plus size={15} />
          </button>
        )}
      </div>

      {!collapsed && (
        <div
          style={{
            padding: '16px 18px',
            fontSize: '14px',
            color: 'var(--color-text-primary)',
            lineHeight: 1.6,
          }}
        >
          {children}
        </div>
      )}
    </div>
  )
}
