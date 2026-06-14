import type { BlockRenderer } from './types'
import { BlockAnnotations } from '@features/annotations/BlockAnnotations'

export const AnnotationRenderer: BlockRenderer = {
  id: 'annotation',
  priority: 6,

  match(_block) {
    // Always match — annotations are optional. The component handles the empty case.
    return true
  },

  renderBeforeContent(ctx) {
    if (!ctx.annotations || ctx.annotations.length === 0) return null

    return (
      <BlockAnnotations
        blockId={ctx.block.id}
        annotations={ctx.annotations}
        indent={ctx.indent ?? 0}
        onAddAnnotation={ctx.onAddAnnotation}
        onReplyAnnotation={ctx.onReplyAnnotation}
        onDeleteAnnotation={ctx.onDeleteAnnotation}
      />
    )
  },
}
