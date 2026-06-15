import type { ReactNode } from 'react'
import type { TaskMarker } from '@shared/types/api'
import type { BlockRenderer, BlockRendererContext } from './types'
import { BlockCheckbox } from './Checkbox'

const TASK_MARKER_CYCLE: (TaskMarker | null)[] = [
  null,
  'Todo',
  'Waiting',
  'Doing',
  'Done',
  'Later',
  'Cancelled',
]

export const TaskRenderer: BlockRenderer = {
  id: 'task',
  priority: 10,

  match(block, strategy) {
    return !!block.marker || strategy === 'task' || block.blockType === 'todo'
  },

  renderBullet(ctx, _defaultBullet) {
    const marker = ctx.block.marker ?? 'Todo'
    return (
      <BlockCheckbox
        marker={marker}
        onChange={() => {
          const currentIdx = TASK_MARKER_CYCLE.indexOf(ctx.block.marker ?? null)
          const nextIdx =
            currentIdx >= 0
              ? (currentIdx + 1) % TASK_MARKER_CYCLE.length
              : 1
          ctx.onCycleMarker(TASK_MARKER_CYCLE[nextIdx])
        }}
      />
    )
  },

  renderBeforeContent() {
    return null
  },

  wrapContent(ctx, content) {
    const { marker } = ctx.block
    const isDimmed = marker === 'Done' || marker === 'Cancelled'
    const isStruck = marker === 'Cancelled'

    return (
      <div
        style={{
          opacity: isDimmed ? 0.6 : 1,
          textDecoration: isStruck ? 'line-through' : 'none',
          width: '100%',
        }}
      >
        {content}
      </div>
    )
  },

  getBlockStyle(_ctx) {
    return undefined
  },
}

export { TASK_MARKER_CYCLE }
