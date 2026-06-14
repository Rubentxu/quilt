import type { ReactNode, CSSProperties } from 'react'
import type { Block } from '@shared/types/api'
import type { BlockStrategyName } from '../useBlockStrategy'
import type { BlockRenderer, BlockRendererContext } from './types'

export class BlockRendererRegistry {
  private renderers: BlockRenderer[] = []

  register(renderer: BlockRenderer): void {
    this.renderers.push(renderer)
    this.renderers.sort((a, b) => a.priority - b.priority)
  }

  /** Get all matching renderers sorted by priority (ascending) */
  getMatching(block: Block, strategy: BlockStrategyName): BlockRenderer[] {
    return this.renderers.filter(r => r.match(block, strategy))
  }

  /** Find highest-priority renderer for exclusive slots (bullet) */
  findHighestPriority(ctx: BlockRendererContext): BlockRenderer | null {
    const matching = this.getMatching(ctx.block, ctx.strategy)
    return matching.length > 0 ? matching[matching.length - 1] : null
  }

  /** Render bullet: highest-priority matching renderer wins, else defaultBullet */
  renderBullet(ctx: BlockRendererContext, defaultBullet: ReactNode): ReactNode {
    for (let i = this.renderers.length - 1; i >= 0; i--) {
      const r = this.renderers[i]
      if (r.match(ctx.block, ctx.strategy) && r.renderBullet) {
        const result = r.renderBullet(ctx, defaultBullet)
        if (result !== null) return result
      }
    }
    return defaultBullet
  }

  /** Collect all renderBeforeContent contributions (highest priority first) */
  renderBeforeContent(ctx: BlockRendererContext): ReactNode[] {
    const results: ReactNode[] = []
    for (let i = this.renderers.length - 1; i >= 0; i--) {
      const r = this.renderers[i]
      if (r.match(ctx.block, ctx.strategy) && r.renderBeforeContent) {
        const result = r.renderBeforeContent(ctx)
        if (result !== null) results.push(result)
      }
    }
    return results
  }

  /** Compose all wrapContent wrappers (lowest priority = outermost) */
  composeContentWrappers(ctx: BlockRendererContext, content: ReactNode): ReactNode {
    let wrapped = content
    for (const r of this.renderers) {
      if (r.match(ctx.block, ctx.strategy) && r.wrapContent) {
        wrapped = r.wrapContent(ctx, wrapped)
      }
    }
    return wrapped
  }

  /** Merge block styles from all matching renderers */
  composeBlockStyle(ctx: BlockRendererContext): CSSProperties {
    const style: CSSProperties = {}
    for (const r of this.renderers) {
      if (r.match(ctx.block, ctx.strategy) && r.getBlockStyle) {
        const s = r.getBlockStyle(ctx)
        if (s) Object.assign(style, s)
      }
    }
    return style
  }

  /** Try content replacement: highest-priority matching renderer with contentReplace wins.
   *  If a renderer returns non-null, that completely replaces the content area — no
   *  composeContentWrappers is called. If no renderer matches, returns defaultContent. */
  replaceContent(ctx: BlockRendererContext, defaultContent: ReactNode): ReactNode {
    for (let i = this.renderers.length - 1; i >= 0; i--) {
      const r = this.renderers[i]
      if (r.match(ctx.block, ctx.strategy) && r.contentReplace) {
        const result = r.contentReplace(ctx)
        if (result !== null) return result
      }
    }
    return defaultContent
  }
}

let _defaultRegistry: BlockRendererRegistry | null = null

export function getDefaultRegistry(): BlockRendererRegistry {
  if (!_defaultRegistry) {
    _defaultRegistry = new BlockRendererRegistry()
  }
  return _defaultRegistry
}

export function createDefaultRegistry(): BlockRendererRegistry {
  const registry = new BlockRendererRegistry()
  return registry
}
