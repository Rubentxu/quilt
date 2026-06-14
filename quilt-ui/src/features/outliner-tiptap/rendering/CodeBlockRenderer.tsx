import type { BlockRenderer } from './types'

export const CodeBlockRenderer: BlockRenderer = {
  id: 'code-block',
  priority: 15,

  match(block) {
    return block.blockType === 'code'
  },

  wrapContent(_ctx, content) {
    return (
      <pre
        style={{
          fontFamily: "'Fira Code', 'Cascadia Code', 'JetBrains Mono', monospace",
          fontSize: '0.875em',
          background: 'var(--color-surface-subtle)',
          borderRadius: 'var(--radius-sm)',
          padding: 'var(--space-2) var(--space-3)',
          margin: '4px 0',
          overflow: 'auto',
          width: '100%',
          lineHeight: 1.5,
          color: 'var(--color-text-primary)',
        }}
      >
        <code>{content}</code>
      </pre>
    )
  },
}
