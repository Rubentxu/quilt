import type { BlockRenderer } from './types'
import type { BlockProperty } from '@shared/types/api'

function readProperty(block: { properties?: BlockProperty[] }, key: string): string | null {
  const prop = block.properties?.find(p => p.key === key)
  if (!prop || prop.value == null) return null
  return String(prop.value)
}

export const ImageRenderer: BlockRenderer = {
  id: 'image',
  priority: 5,

  match(block) {
    return block.blockType === 'image'
  },

  contentReplace(ctx) {
    const imageUrl = readProperty(ctx.block, 'image-url')
    const imageAlt = readProperty(ctx.block, 'image-alt') ?? ctx.block.content ?? 'Image'

    if (!imageUrl) {
      return (
        <div
          data-testid="image-placeholder"
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 'var(--space-2)',
            padding: 'var(--space-3)',
            background: 'var(--color-surface-subtle)',
            border: '1px dashed var(--color-border)',
            borderRadius: 'var(--radius-sm)',
            color: 'var(--color-text-muted)',
            fontSize: '13px',
            flex: 1,
            minWidth: 0,
          }}
        >
          <span aria-hidden="true">🖼️</span>
          <span>No image URL set. Add an `image-url::` property.</span>
        </div>
      )
    }

    return (
      <div
        data-testid="image-preview"
        style={{
          flex: 1,
          minWidth: 0,
          display: 'flex',
          flexDirection: 'column',
          gap: 'var(--space-1)',
        }}
      >
        <img
          src={imageUrl}
          alt={imageAlt}
          style={{
            maxWidth: '100%',
            maxHeight: '400px',
            borderRadius: 'var(--radius-md)',
            objectFit: 'contain',
            background: 'var(--color-surface-subtle)',
          }}
          loading="lazy"
          onError={(e) => {
            const target = e.currentTarget
            target.style.display = 'none'
            const parent = target.parentElement
            if (parent) {
              const error = document.createElement('div')
              error.setAttribute('data-testid', 'image-error')
              error.style.cssText = 'display:flex;align-items:center;gap:8px;padding:16px;background:var(--color-danger-subtle, rgba(220,38,38,0.08));color:var(--color-danger, #dc2626);border-radius:6px;font-size:13px;'
              error.innerHTML = '<span>⚠️</span><span>Failed to load image</span>'
              parent.appendChild(error)
            }
          }}
        />
        <span
          style={{
            fontSize: '11px',
            color: 'var(--color-text-muted)',
            paddingLeft: '4px',
          }}
        >
          {imageAlt}
        </span>
      </div>
    )
  },
}
