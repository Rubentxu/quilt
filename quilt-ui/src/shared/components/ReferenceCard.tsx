/**
 * ReferenceCard — DESIGN.md §9.7
 *
 * Tarjeta para referencias vinculadas. Aparece dentro del contenido
 * (debajo de un bloque origen) para mostrar otras páginas o
 * documentos que están relacionados.
 *
 * Estructura:
 *   [icono]  Título de referencia
 *           meta1: valor
 *           meta2: valor
 *           [abrir]  [más acciones]
 */
import { type ReactNode, useState } from 'react'
import { FileText, ExternalLink, MoreHorizontal, Copy } from 'lucide-react'
import toast from 'react-hot-toast'

export interface ReferenceMeta {
  key: string
  value: string
}

interface ReferenceCardProps {
  title: string
  metas?: ReferenceMeta[]
  href?: string
  icon?: ReactNode
  onOpen?: () => void
  /** Contenido editable envuelto dentro de la card (tipicamente un BlockRow). */
  children?: ReactNode
}

export function ReferenceCard({
  title,
  metas = [],
  href,
  icon,
  onOpen,
  children,
}: ReferenceCardProps) {
  const [hover, setHover] = useState(false)
  const [menuOpen, setMenuOpen] = useState(false)

  const iconNode = icon ?? (
    <div
      style={{
        width: '36px',
        height: '36px',
        borderRadius: 'var(--radius-md)',
        background: 'rgba(37, 99, 235, 0.08)',
        color: 'var(--color-primary)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        flexShrink: 0,
      }}
    >
      <FileText size={18} aria-hidden="true" />
    </div>
  )

  async function copyLink() {
    const url = href ?? window.location.href
    try {
      await navigator.clipboard.writeText(url)
      toast.success('Link copied')
    } catch {
      toast.error('Failed to copy link')
    }
    setMenuOpen(false)
  }

  return (
    <div
      data-testid="reference-card"
      onMouseEnter={() => setHover(true)}
      onMouseLeave={() => {
        setHover(false)
        setMenuOpen(false)
      }}
      style={{
        display: 'flex',
        gap: 'var(--space-3)',
        padding: 'var(--space-4)',
        background: 'var(--color-surface)',
        border: 'none',
        borderRadius: 'var(--radius-lg)',
        boxShadow: hover
          ? '0 12px 32px rgba(15, 23, 42, 0.06)'
          : 'var(--shadow-sm)',
        transition:
          'box-shadow var(--motion-normal) var(--ease-standard), transform var(--motion-normal) var(--ease-standard)',
        transform: hover ? 'translateY(-1px)' : 'none',
      }}
    >
      {iconNode}

      <div style={{ flex: 1, minWidth: 0 }}>
        {href ? (
          <a
            href={href}
            style={{
              color: 'var(--color-link)',
              fontSize: '14px',
              fontWeight: 600,
              textDecoration: 'none',
              display: 'block',
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
            }}
          >
            {title}
          </a>
        ) : (
          <div
            style={{
              color: 'var(--color-text-primary)',
              fontSize: '14px',
              fontWeight: 600,
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
            }}
          >
            {title}
          </div>
        )}

        {metas.length > 0 && (
          <div
            style={{
              display: 'flex',
              flexDirection: 'column',
              gap: '4px',
              marginTop: '6px',
            }}
          >
            {metas.map((m) => (
              <div
                key={m.key}
                style={{
                  display: 'grid',
                  gridTemplateColumns: '120px 1fr',
                  gap: 'var(--space-3)',
                  fontSize: '12px',
                  color: 'var(--color-text-secondary)',
                }}
              >
                <span style={{ color: 'var(--color-text-muted)' }}>{m.key}:</span>
                <span
                  style={{
                    overflow: 'hidden',
                    textOverflow: 'ellipsis',
                    whiteSpace: 'nowrap',
                  }}
                >
                  {m.value}
                </span>
              </div>
            ))}
          </div>
        )}

        {children && (
          <div style={{ marginTop: 'var(--space-3)' }}>{children}</div>
        )}
      </div>

      <div style={{ display: 'flex', gap: '4px', alignItems: 'flex-start' }}>
        {onOpen && (
          <button
            type="button"
            aria-label="Open reference"
            onClick={onOpen}
            className="ghost-icon-button"
            style={{
              width: '32px',
              height: '32px',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              cursor: 'pointer',
            }}
          >
            <ExternalLink size={15} />
          </button>
        )}
        <div style={{ position: 'relative' }}>
          <button
            type="button"
            aria-label="More actions"
            aria-haspopup="menu"
            aria-expanded={menuOpen}
            onClick={() => setMenuOpen((v) => !v)}
            className="ghost-icon-button"
            style={{
              width: '32px',
              height: '32px',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              cursor: 'pointer',
            }}
          >
            <MoreHorizontal size={15} />
          </button>
          {menuOpen && (
            <div
              role="menu"
              style={{
                position: 'absolute',
                top: '100%',
                right: 0,
                marginTop: '4px',
                background: 'var(--color-surface-elevated)',
                border: '1px solid var(--color-border)',
                borderRadius: 'var(--radius-md)',
                boxShadow: 'var(--shadow-md)',
                padding: '4px',
                minWidth: '160px',
                zIndex: 50,
              }}
            >
              <button
                type="button"
                role="menuitem"
                onClick={copyLink}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 'var(--space-2)',
                  width: '100%',
                  padding: '8px 10px',
                  background: 'transparent',
                  border: 'none',
                  borderRadius: 'var(--radius-sm)',
                  cursor: 'pointer',
                  fontSize: '13px',
                  color: 'var(--color-text-primary)',
                  textAlign: 'left',
                  fontFamily: 'inherit',
                }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.background = 'var(--color-surface-subtle)'
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.background = 'transparent'
                }}
              >
                <Copy size={13} />
                Copy link
              </button>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
