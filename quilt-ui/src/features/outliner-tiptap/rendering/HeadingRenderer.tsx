import type { BlockRenderer } from './types'

const HEADING_STYLES: Record<string, { tag: keyof HTMLElementTagNameMap; fontSize: string; fontWeight: number }> = {
  heading1: { tag: 'h1', fontSize: '2em', fontWeight: 700 },
  heading2: { tag: 'h2', fontSize: '1.5em', fontWeight: 600 },
  heading3: { tag: 'h3', fontSize: '1.17em', fontWeight: 600 },
}

export const HeadingRenderer: BlockRenderer = {
  id: 'heading',
  priority: 20,

  match(block) {
    return block.blockType === 'heading1' || block.blockType === 'heading2' || block.blockType === 'heading3'
  },

  wrapContent(ctx, content) {
    const style = HEADING_STYLES[ctx.block.blockType]
    if (!style) return content

    const Tag = style.tag as keyof HTMLElementTagNameMap
    return (
      <Tag
        style={{
          fontSize: style.fontSize,
          fontWeight: style.fontWeight,
          margin: '0.25em 0',
          lineHeight: 1.3,
          width: '100%',
          color: 'var(--color-text-primary)',
        }}
      >
        {content}
      </Tag>
    )
  },
}
