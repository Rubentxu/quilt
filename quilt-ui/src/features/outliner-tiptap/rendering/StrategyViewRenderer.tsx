import { lazy, Suspense } from 'react'
import type { BlockRenderer } from './types'

const SavedViewBlock = lazy(() =>
  import('@features/view/SavedViewBlock').then(m => ({ default: m.SavedViewBlock })),
)

function SavedViewFallback() {
  return (
    <div
      data-testid="saved-view-block"
      style={{
        padding: 'var(--space-2) 0',
        color: 'var(--color-text-muted)',
        fontSize: '13px',
      }}
    >
      Loading view...
    </div>
  )
}

export const StrategyViewRenderer: BlockRenderer = {
  id: 'strategy-view',
  priority: 100, // Highest priority — replaces content entirely when view strategy matches

  match(_block, strategy) {
    return strategy === 'view'
  },

  contentReplace(ctx) {
    return (
      <div
        key="view"
        data-testid="block-view-content"
        style={{
          flex: 1,
          minWidth: 0,
        }}
      >
        <Suspense fallback={<SavedViewFallback />}>
          <SavedViewBlock block={ctx.block} allBlocks={ctx.allBlocks ?? []} />
        </Suspense>
      </div>
    )
  },
}
