import { useState, useEffect, useRef } from 'react'
import { api } from '@core/api-client'

interface HoverPreviewProps {
  type: 'page' | 'block'
  target: string
  anchorRect: DOMRect | null
  onClose: () => void
  onMouseEnter?: () => void
  onMouseLeave?: () => void
}

export function HoverPreview({ type, target, anchorRect, onClose, onMouseEnter, onMouseLeave }: HoverPreviewProps) {
  const [content, setContent] = useState<{ title: string; preview: string } | null>(null)
  const [loading, setLoading] = useState(true)
  const containerRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!target) return
    let mounted = true

    const load = async () => {
      try {
        if (type === 'page') {
          const page = await api.getPage(target)
          if (!mounted) return
          const blocks = await api.getPageBlocks(target)
          const preview = blocks
            .slice(0, 3)
            .map(b => b.content)
            .filter(Boolean)
            .join('\n')
            .substring(0, 200)
          setContent({ title: page.title || page.name, preview })
        } else {
          // Block: show the block reference info
          setContent({ title: 'Block reference', preview: `ID: ${target.slice(0, 8)}…` })
        }
      } catch {
        if (!mounted) return
        setContent({ title: type === 'page' ? target : 'Block', preview: '(not found)' })
      } finally {
        if (mounted) setLoading(false)
      }
    }

    load()
    return () => { mounted = false }
  }, [type, target])

  // Click outside closes popover
  useEffect(() => {
    function handleClick(e: MouseEvent) {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        onClose()
      }
    }
    document.addEventListener('mousedown', handleClick)
    return () => document.removeEventListener('mousedown', handleClick)
  }, [onClose])

  if (!anchorRect) return null

  // Position below the anchor
  const top = anchorRect.bottom + 8
  const left = Math.min(anchorRect.left, window.innerWidth - 320)

  return (
    <div
      ref={containerRef}
      onMouseEnter={onMouseEnter}
      onMouseLeave={onMouseLeave}
      style={{
        position: 'fixed',
        top,
        left,
        zIndex: 300,
        background: 'var(--color-surface)',
        border: '1px solid var(--color-border)',
        borderRadius: 'var(--radius-md)',
        boxShadow: 'var(--shadow-lg)',
        padding: 'var(--space-3)',
        minWidth: '240px',
        maxWidth: '320px',
        fontSize: '13px',
      }}
    >
      {loading ? (
        <div style={{ color: 'var(--color-text-muted)' }}>Loading…</div>
      ) : content ? (
        <>
          <div
            style={{
              fontWeight: 600,
              marginBottom: 'var(--space-2)',
              color: 'var(--color-text-primary)',
              display: 'flex',
              alignItems: 'center',
              gap: '4px',
            }}
          >
            {type === 'page' ? '📄' : '#'} {content.title}
          </div>
          {content.preview && (
            <div
              style={{
                color: 'var(--color-text-secondary)',
                fontSize: '12px',
                lineHeight: 1.5,
                maxHeight: '100px',
                overflow: 'hidden',
              }}
            >
              {content.preview}
            </div>
          )}
          <div
            style={{
              marginTop: 'var(--space-2)',
              paddingTop: 'var(--space-2)',
              borderTop: '1px solid var(--color-border)',
              fontSize: '11px',
              color: 'var(--color-text-muted)',
            }}
          >
            <kbd style={{ padding: '1px 4px', background: 'var(--color-surface-subtle)', borderRadius: '2px' }}>Click</kbd> to open ·
            <kbd style={{ padding: '1px 4px', background: 'var(--color-surface-subtle)', borderRadius: '2px' }}>Shift+Click</kbd> in sidebar ·
            <kbd style={{ padding: '1px 4px', background: 'var(--color-surface-subtle)', borderRadius: '2px' }}>Cmd+Click</kbd> new tab
          </div>
        </>
      ) : null}
    </div>
  )
}
