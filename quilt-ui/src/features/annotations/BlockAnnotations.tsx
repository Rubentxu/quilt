/**
 * BlockAnnotations — annotation badge + inline thread for a single block row.
 *
 * Extracted from BlockRow to keep it under 1K lines (thermo-nuclear audit 2026-06-11).
 * Renders the annotation count badge and the pending annotation thread below the block.
 *
 * Feature flag: `window.__QUILT_ANNOTATIONS_ENABLED__` (defaults to true).
 */

import { AnnotationBadge } from './AnnotationBadge'
import { AnnotationRow } from './AnnotationRow'
import { countOpenAnnotations } from './annotationUtils'
import type { Annotation } from '@shared/types/api'

export interface BlockAnnotationsProps {
  blockId: string
  annotations?: Annotation[]
  indent: number
  onAddAnnotation?: (blockId: string, scope: 'block') => void
  onReplyAnnotation?: (annotationId: string) => void
  onDeleteAnnotation?: (annotationId: string) => void
}

export function BlockAnnotations({
  blockId,
  annotations,
  indent,
  onAddAnnotation,
  onReplyAnnotation,
  onDeleteAnnotation,
}: BlockAnnotationsProps) {
  const enabled = typeof window !== 'undefined'
    ? (window as any).__QUILT_ANNOTATIONS_ENABLED__ !== false
    : true

  const openCount = enabled ? countOpenAnnotations(annotations ?? []) : 0

  return (
    <>
      {/* Badge — shows count of pending + in-progress annotations */}
      <AnnotationBadge
        count={openCount}
        onClick={
          onAddAnnotation
            ? () => onAddAnnotation(blockId, 'block')
            : undefined
        }
        testId={`annotation-badge-${blockId}`}
      />

      {/* Thread — rendered inline below the block when annotations exist */}
      {enabled && openCount > 0 && (
        <div
          data-testid={`annotation-thread-${blockId}`}
          style={{
            marginLeft: `${indent * 24 + 32}px`,
            marginTop: 'var(--space-1)',
            padding: 'var(--space-2) var(--space-3)',
            background: 'var(--color-surface-subtle)',
            borderLeft: '2px solid var(--color-accent)',
            borderRadius: 'var(--radius-sm)',
          }}
        >
          <div
            style={{
              fontSize: '11px',
              fontWeight: 600,
              color: 'var(--color-text-muted)',
              marginBottom: 'var(--space-1)',
              display: 'flex',
              alignItems: 'center',
              gap: 'var(--space-1)',
            }}
          >
            <span aria-hidden="true">💬</span>
            <span>{openCount} annotation{openCount !== 1 ? 's' : ''}</span>
          </div>
          {(annotations ?? [])
            .filter(a => a.status === 'pending' || a.status === 'in_progress')
            .map(a => (
              <AnnotationRow
                key={a.id}
                annotation={a}
                onReply={onReplyAnnotation ? () => onReplyAnnotation(a.id) : undefined}
                onDelete={onDeleteAnnotation ? () => onDeleteAnnotation(a.id) : undefined}
              />
            ))}
        </div>
      )}
    </>
  )
}
