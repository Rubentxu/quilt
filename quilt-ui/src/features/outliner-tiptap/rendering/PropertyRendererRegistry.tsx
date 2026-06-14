import type { ReactNode } from 'react'
import { Calendar, Clock, FileText, Tag, User } from 'lucide-react'

export interface PropertyRenderer {
  key: string
  match: (propKey: string) => boolean
  render: (value: unknown, propKey: string) => ReactNode
}

function renderChip(
  content: ReactNode,
  options?: {
    background?: string
    color?: string
    border?: string
    gap?: string
    fontWeight?: number
    padding?: string
  },
) {
  return (
    <span
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: options?.gap ?? '4px',
        padding: options?.padding ?? '1px 8px',
        borderRadius: 'var(--radius-pill)',
        background: options?.background ?? 'var(--color-surface-subtle)',
        color: options?.color ?? 'var(--color-text-secondary)',
        fontSize: '11px',
        fontWeight: options?.fontWeight ?? 500,
        marginRight: '4px',
        verticalAlign: 'middle',
        border: options?.border,
      }}
    >
      {content}
    </span>
  )
}

function renderDefaultProperty(value: unknown, propKey: string): ReactNode {
  return renderChip(
    <>
      <span style={{ color: 'var(--color-text-muted)', fontSize: '10px' }}>{propKey}</span>
      <span>{String(value ?? '')}</span>
    </>,
    { border: '1px solid var(--color-border)' },
  )
}

export class PropertyRendererRegistry {
  private renderers: PropertyRenderer[] = []

  register(renderer: PropertyRenderer): void {
    this.renderers.push(renderer)
  }

  getRenderer(propKey: string): PropertyRenderer | undefined {
    return this.renderers.find(renderer => renderer.match(propKey))
  }

  render(value: unknown, propKey: string): ReactNode {
    return this.getRenderer(propKey)?.render(value, propKey) ?? renderDefaultProperty(value, propKey)
  }
}

export const propertyRendererRegistry = new PropertyRendererRegistry()

propertyRendererRegistry.register({
  key: 'status',
  match: (propKey) => propKey === 'status',
  render: (value) => {
    const safeValue = String(value ?? '')
    const colors: Record<string, { bg: string; fg: string }> = {
      todo: { bg: 'var(--color-info-subtle)', fg: 'var(--color-info)' },
      doing: { bg: 'var(--color-warning-subtle)', fg: 'var(--color-warning)' },
      done: { bg: 'var(--color-success-subtle)', fg: 'var(--color-success)' },
      now: { bg: 'var(--color-accent-subtle)', fg: 'var(--color-accent)' },
      later: { bg: 'var(--color-surface-subtle)', fg: 'var(--color-text-muted)' },
      cancelled: { bg: 'var(--color-danger-subtle)', fg: 'var(--color-danger)' },
    }
    const color = colors[safeValue.toLowerCase()] ?? {
      bg: 'var(--color-surface-subtle)',
      fg: 'var(--color-text-secondary)',
    }

    return renderChip(safeValue.toUpperCase(), {
      background: color.bg,
      color: color.fg,
      fontWeight: 600,
    })
  },
})

propertyRendererRegistry.register({
  key: 'priority',
  match: (propKey) => propKey === 'priority',
  render: (value) => {
    const safeValue = String(value ?? '')
    const colors: Record<string, string> = {
      a: 'var(--color-danger)',
      b: 'var(--color-warning)',
      c: 'var(--color-info)',
    }
    const color = colors[safeValue.toLowerCase()] ?? 'var(--color-text-secondary)'

    return renderChip(`P${safeValue.toUpperCase()}`, {
      background: `${color}20`,
      color,
      border: `1px solid ${color}40`,
      fontWeight: 700,
      padding: '1px 6px',
      gap: '0',
    })
  },
})

propertyRendererRegistry.register({
  key: 'tags',
  match: (propKey) => propKey === 'tags',
  render: (value) => {
    const tags = String(value ?? '')
      .split(',')
      .map(tag => tag.trim())
      .filter(Boolean)

    return (
      <span style={{ display: 'inline', marginRight: '4px' }}>
        {tags.map((tag, index) => (
          <span
            key={`${tag}-${index}`}
            style={{
              display: 'inline-flex',
              alignItems: 'center',
              padding: '1px 6px',
              borderRadius: 'var(--radius-pill)',
              background: 'var(--color-info-subtle)',
              color: 'var(--color-info)',
              fontSize: '11px',
              fontWeight: 500,
              marginRight: '4px',
              gap: '3px',
            }}
          >
            <Tag size={10} />
            {tag}
          </span>
        ))}
      </span>
    )
  },
})

propertyRendererRegistry.register({
  key: 'deadline',
  match: (propKey) => propKey === 'deadline',
  render: (value) => renderChip(<><Calendar size={12} />{String(value ?? '')}</>),
})

propertyRendererRegistry.register({
  key: 'scheduled',
  match: (propKey) => propKey === 'scheduled',
  render: (value) => renderChip(<><Clock size={12} />{String(value ?? '')}</>),
})

propertyRendererRegistry.register({
  key: 'created_by',
  match: (propKey) => propKey === 'created_by',
  render: (value) => renderChip(<><User size={12} />{String(value ?? '')}</>),
})

propertyRendererRegistry.register({
  key: 'template',
  match: (propKey) => propKey === 'template',
  render: (value) => renderChip(<><FileText size={12} />{String(value ?? '')}</>),
})
