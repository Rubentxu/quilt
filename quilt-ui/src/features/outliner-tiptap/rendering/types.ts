import type { CSSProperties, ReactNode } from 'react'
import type { Annotation, Block, TaskMarker } from '@shared/types/api'
import type { BlockStrategyName } from '../useBlockStrategy'

/** Context passed to every renderer method */
export interface BlockRendererContext {
  block: Block
  strategy: BlockStrategyName
  /** Optimistic local state update */
  onUpdate: (block: Block) => void
  /** Cycle the task marker and persist to API */
  onCycleMarker: (nextMarker: TaskMarker | null) => void
  /** Indent level of the block row (0 = root). Used for nested UI like threads. */
  indent?: number
  /** Annotations for the current block (from the annotation API). */
  annotations?: Annotation[]
  /** Add a new annotation to this block. */
  onAddAnnotation?: (blockId: string, scope: 'block') => void
  /** Reply to an annotation. */
  onReplyAnnotation?: (annotationId: string) => void
  /** Delete an annotation. */
  onDeleteAnnotation?: (annotationId: string) => void
  /** All blocks on the current page (needed by view renderers that query from siblings). */
  allBlocks?: Block[]
}

/**
 * A composable block renderer.
 *
 * Each renderer controls specific rendering "slots" in BlockRow.
 * Multiple renderers can match the same block — their contributions compose.
 * For exclusive slots (renderBullet), highest priority wins.
 * For compositional slots (wrapContent), all matching renderers compose (outermost = lowest priority).
 */
export interface BlockRenderer {
  /** Unique identifier */
  id: string

  /** Return true if this renderer should activate */
  match: (block: Block, strategy: BlockStrategyName) => boolean

  /** Higher = wins for exclusive slots, innermost for wrappers */
  priority: number

  /**
   * Override the bullet/chevron area.
   * Return null to use default bullet (dot or chevron).
   * Highest-priority matching renderer wins.
   */
  renderBullet?: (ctx: BlockRendererContext, defaultBullet: ReactNode) => ReactNode | null

  /**
   * Render elements between bullet and content area.
   * All matching renderers contribute (ordered by priority, highest first).
   * Return null to contribute nothing.
   */
  renderBeforeContent?: (ctx: BlockRendererContext) => ReactNode | null

  /**
   * Wrap the content area. MUST return children (can add wrapper div).
   * All matching renderers compose (outermost = lowest priority).
   */
  wrapContent?: (ctx: BlockRendererContext, content: ReactNode) => ReactNode

  /**
   * Additional inline styles for the block row container.
   * Merged from all matching renderers (later = higher priority overrides).
   */
  getBlockStyle?: (ctx: BlockRendererContext) => CSSProperties | undefined

  /**
   * Replace the entire content area. Highest-priority matching renderer wins.
   * When a renderer returns non-null, composeContentWrappers is SKIPPED entirely.
   * Used by view renderers (SavedViewBlock) that own the full content region.
   */
  contentReplace?: (ctx: BlockRendererContext) => ReactNode | null
}
