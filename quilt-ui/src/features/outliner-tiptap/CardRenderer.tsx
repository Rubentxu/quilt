/**
 * CardRenderer — ADR-0007
 *
 * Data-driven card wrapper for template-activated blocks. Replaces the
 * hardcoded `ReferenceCard` and `ContentCard` components. The shape,
 * icon, CSS class, and visual treatment are all derived from the
 * template page that the block references via `template::`.
 *
 * Three shapes in V1:
 *   - `reference` — flat card with meta table + open/copy actions
 *     (ex-ReferenceCard, DESIGN.md §9.7)
 *   - `content`   — collapsible card with header + content body
 *     (ex-ContentCard, DESIGN.md §9.8)
 *   - `inline`    — no card wrapper, just decorations (icon, CSS class)
 *
 * The user can create new templates (with their own `card-shape::`,
 * `icon::`, `cssclass::`) and they automatically get rendered here
 * without touching code.
 */

import { type ReactNode, useState } from 'react'
import { FileText, ExternalLink, MoreHorizontal, Copy, ChevronDown, ChevronRight } from 'lucide-react'
import toast from 'react-hot-toast'

// ── Types ──────────────────────────────────────────────────────────

export type CardShape = 'reference' | 'content' | 'inline' | 'kanban-card' | 'timeline-card'

export interface BlockCard {
  /** The card's visual shape. */
  shape: CardShape
  /** Icon (emoji or text) to display as the block's decoration. */
  icon?: string
  /** CSS class(es) to apply to the wrapper. */
  cssclass?: string
  /** The template's display name (the page name, e.g., "meeting-notes"). */
  templateName: string
}

export interface CardMeta {
  key: string
  value: string
}

interface CardRendererProps {
  card: BlockCard
  title: string
  metas?: CardMeta[]
  href?: string
  children: ReactNode
  onOpen?: () => void
}

// ── Sub-renderers ──────────────────────────────────────────────────

/** reference-shape: flat card with icon, title, meta table, actions. */
function ReferenceShape({ card, title, metas, href, children, onOpen }: CardRendererProps) {
  const [hover, setHover] = useState(false)
  const [menuOpen, setMenuOpen] = useState(false)

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
      data-testid="card-renderer"
      data-shape={card.shape}
      data-template={card.templateName}
      onMouseEnter={() => setHover(true)}
      onMouseLeave={() => {
        setHover(false)
        setMenuOpen(false)
      }}
      className={card.cssclass}
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
      {card.icon ? (
        <div
          aria-hidden="true"
          style={{
            width: '36px',
            height: '36px',
            borderRadius: 'var(--radius-md)',
            background: 'rgba(37, 99, 235, 0.08)',
            color: 'var(--color-primary)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            fontSize: '20px',
            flexShrink: 0,
          }}
        >
          {card.icon}
        </div>
      ) : (
        <div
          aria-hidden="true"
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
      )}

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

        {metas && metas.length > 0 && (
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

/** content-shape: collapsible card with header + body. */
function ContentShape({ card, title, children }: CardRendererProps) {
  const [collapsed, setCollapsed] = useState(false)

  return (
    <div
      data-testid="card-renderer"
      data-shape={card.shape}
      data-template={card.templateName}
      className={card.cssclass}
      style={{
        background: 'var(--color-surface)',
        border: 'none',
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
        {card.icon ? (
          <span
            aria-hidden="true"
            style={{
              color: 'var(--color-text-muted)',
              fontSize: '15px',
              flexShrink: 0,
            }}
          >
            {card.icon}
          </span>
        ) : (
          <FileText
            size={15}
            style={{ color: 'var(--color-text-muted)', flexShrink: 0 }}
          />
        )}
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
        </div>
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

/** inline-shape: no card wrapper, just the children with a className. */
function InlineShape({ card, children }: CardRendererProps) {
  // T-22: when invoked from the V1 placeholder path for
  // kanban-card / timeline-card, `card.shape` will be the original
  // shape name and we use that. For the default inline case it's
  // 'inline'. Either way, we render the actual card.shape value.
  return (
    <div
      data-testid="card-renderer"
      data-shape={card.shape}
      data-template={card.templateName}
      className={card.cssclass}
      style={{ display: 'contents' }}
    >
      {children}
    </div>
  )
}

// ── Main renderer ──────────────────────────────────────────────────

/**
 * Renders a card around children based on the card shape.
 *
 * If `card` is null, returns `children` unchanged (the block has no
 * template activation and renders as a normal outliner block).
 */
export function CardRenderer(props: CardRendererProps) {
  const { card } = props
  if (!card) return <>{props.children}</>

  switch (card.shape) {
    case 'reference':
      return <ReferenceShape {...props} />
    case 'content':
      return <ContentShape {...props} />
    case 'inline':
      return <InlineShape {...props} />
    case 'kanban-card':
    case 'timeline-card':
      // V1 placeholder: render with the original shape name preserved
      // on the data-shape attribute so user CSS can hook into it. The
      // actual kanban board / timeline wrappers will be added in a
      // follow-up. We use the same wrapper as InlineShape but override
      // data-shape via the card prop's shape field.
      return <InlineShape {...props} />
    default:
      // Unknown shape — render as inline with a console warning so the
      // user/dev knows their template page has an invalid card-shape::
      // value but the block is still readable.
      // eslint-disable-next-line no-console
      console.warn(
        `[CardRenderer] Unknown card-shape "${card.shape}" on template "${card.templateName}". Falling back to inline.`,
      )
      return <InlineShape {...props} />
  }
}
